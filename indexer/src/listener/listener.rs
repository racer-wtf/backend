use std::sync::Arc;
use std::time::Duration;

use bigdecimal::BigDecimal;
use database::{Cycle, Database, Vote};
use ethers::{
    contract::abigen,
    providers::{Middleware, Provider, StreamExt, Ws},
    types::{H160, U64},
};
use num_bigint::BigInt;

use super::bytes::{bigdecimal_to_bytes, bytes_to_bigdecimal};

abigen!(
    Racer,
    r#"[
        event CycleCreated(address indexed creator, uint256 indexed id, uint256, uint256, uint256)
        event VotePlaced(address indexed placer, uint256 indexed voteId, uint256 indexed cycleId, bytes4 symbol, uint256 amount, uint256 placement)
        event VoteClaimed(address indexed placer, uint256 indexed id, uint256 reward)
    ]"#,
);

type RacerContract = Racer<Provider<Ws>>;

pub struct Listener {
    contract_address: String,
    starting_block: u64,
    rpc_url: String,
    database: Database,
    reorg_threshold: BigInt,
}

impl Listener {
    pub fn new(database: Database) -> Self {
        Self {
            contract_address: String::new(),
            starting_block: 0,
            rpc_url: String::new(),
            database,
            reorg_threshold: BigInt::from(7),
        }
    }

    pub fn with_contract_address(mut self, contract_address: String) -> Self {
        self.contract_address = contract_address;
        self
    }

    pub fn with_starting_block(mut self, starting_block: u64) -> Self {
        self.starting_block = starting_block;
        self
    }

    pub fn with_rpc_url(mut self, rpc_url: String) -> Self {
        self.rpc_url = rpc_url;
        self
    }

    #[allow(dead_code)]
    pub fn with_reorg_threshold(mut self, threshold: usize) -> Self {
        self.reorg_threshold = BigInt::from(threshold);
        self
    }

    /// Starts listening to the provider
    pub async fn start(&self) {
        let provider = Provider::<Ws>::connect(self.rpc_url.clone())
            .await
            .expect("Could not connect to RPC")
            .interval(Duration::from_secs(2));
        let client = Arc::new(provider);
        let contract = Racer::new(
            self.contract_address
                .parse::<H160>()
                .expect("Invalid contract address"),
            client.clone(),
        );

        self.listen_blocks(client, &contract).await;
    }

    /// Watches for new blocks and triggers indexing
    async fn listen_blocks(&self, client: Arc<Provider<Ws>>, contract: &RacerContract) {
        // let mut stream = events.subscribe_with_meta().await.unwrap();
        let mut stream = client.watch_blocks().await.unwrap();

        tracing::info!("listening for events on {}", self.contract_address);

        while let Some(block) = stream.next().await {
            let Ok(Some(block)) = client.get_block(block).await else { return };
            let Some(block_number) = block.number else { return };
            tracing::trace!("found block number: {}", block_number.to_string());
            let Ok(chain_id) = client.get_chainid().await else { return };

            let Ok(current_height) = self
                .database
                .get_block_height(bytes_to_bigdecimal(chain_id))
                .await else { return };

            let reorg_height = bytes_to_bigdecimal(block_number) - &self.reorg_threshold;

            // this picks which block to index from
            // step 1 - finds the max of either last indexed block or configured START_HEIGHT
            // step 2 - finds the min of the previous value or current RPC height - 7
            let useful_height =
                BigDecimal::max(current_height, BigDecimal::from(self.starting_block));
            let useful_height = BigDecimal::min(useful_height, reorg_height.clone());

            self.index(&contract, bigdecimal_to_bytes(useful_height.clone()))
                .await;

            match self
                .database
                .set_block_height(bytes_to_bigdecimal(chain_id), reorg_height.clone())
                .await
            {
                Ok(_) => tracing::info!("updated block height to {}", reorg_height.to_string()),
                Err(e) => tracing::error!("could not update block height: {:?}", e),
            }
        }
    }

    /// Given a block start, it saves all the events from that block to the database
    async fn index(&self, contract: &RacerContract, from_block: U64) {
        tracing::trace!("indexing from block {}", from_block.to_string());
        let events = contract.events().from_block(from_block).query().await;

        if let Ok(events) = events {
            let Ok(mut tx) = self.database.start_transaction().await else { return };

            for event in events {
                match event {
                    RacerEvents::CycleCreatedFilter(event) => {
                        self.create_cycle(&mut tx, event, from_block).await
                    }
                    RacerEvents::VotePlacedFilter(event) => {
                        self.create_vote(&mut tx, event, from_block).await
                    }
                    RacerEvents::VoteClaimedFilter(event) => {
                        self.claim_vote(&mut tx, event, from_block).await
                    }
                }
            }

            match tx.commit().await {
                Ok(_) => tracing::info!("indexed from block {}", from_block.to_string()),
                Err(e) => tracing::error!("could not commit transaction: {:?}", e),
            }
        }
    }

    /// Saves a cycle to the database
    async fn create_cycle(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        event: CycleCreatedFilter,
        block_number: U64,
    ) {
        let result = self
            .database
            .delete_cycles(tx, bytes_to_bigdecimal(block_number))
            .await;

        match result {
            Ok(_) => tracing::info!("removed stale cycles: {:?}", event),
            Err(e) => tracing::error!("error removing stale cycles: {:?}", e),
        }

        let result = self
            .database
            .create_cycle(
                tx,
                Cycle {
                    id: bytes_to_bigdecimal(event.id),
                    block_number: bytes_to_bigdecimal(block_number),
                    creator: format!("{:#032x}", event.creator),
                    starting_block: bytes_to_bigdecimal(event.p2),
                    block_length: bytes_to_bigdecimal(event.p3),
                    vote_price: bytes_to_bigdecimal(event.p4),
                    balance: BigDecimal::default(),
                },
            )
            .await;

        match result {
            Ok(_) => tracing::info!("cycle saved to db: {:?}", event),
            Err(e) => tracing::error!("error saving cycle to db: {:?}", e),
        }
    }

    /// Saves a vote to the database
    async fn create_vote(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        event: VotePlacedFilter,
        block_number: U64,
    ) {
        let result = self
            .database
            .delete_votes(tx, bytes_to_bigdecimal(block_number))
            .await;

        match result {
            Ok(_) => tracing::info!("removed stale votes: {:?}", event),
            Err(e) => tracing::error!("error removing stale votes: {:?}", e),
        }

        let result = self
            .database
            .create_vote(
                tx,
                Vote {
                    id: bytes_to_bigdecimal(event.vote_id),
                    block_number: bytes_to_bigdecimal(block_number),
                    cycle_id: bytes_to_bigdecimal(event.cycle_id),
                    placer: format!("{:#032x}", event.placer),
                    symbol: event.symbol,
                    amount: bytes_to_bigdecimal(event.amount),
                    placement: bytes_to_bigdecimal(event.placement),
                },
            )
            .await;

        match result {
            Ok(_) => tracing::info!("vote saved to db: {:?}", event),
            Err(e) => tracing::error!("error saving vote to db: {:?}", e),
        }
    }

    /// Saves a vote claim to the database
    async fn claim_vote(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        event: VoteClaimedFilter,
        block_number: U64,
    ) {
        let result = self
            .database
            .reset_vote_claims(tx, bytes_to_bigdecimal(block_number))
            .await;

        match result {
            Ok(_) => tracing::info!("removed stale vote claims: {:?}", event),
            Err(e) => tracing::error!("error removing stale vote claims: {:?}", e),
        }

        let result = self
            .database
            .claim_vote(tx, bytes_to_bigdecimal(event.id))
            .await;

        match result {
            Ok(_) => tracing::info!("vote claim saved to db: {:?}", event),
            Err(e) => tracing::error!("error saving vote claim to db: {:?}", e),
        }
    }
}
