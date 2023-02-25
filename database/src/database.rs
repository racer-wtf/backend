use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::types::BigDecimal;
use sqlx::{Error, Result as SqlxResult};

use super::models::{Cycle, Vote};

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(url: &str) -> Result<Self, Error> {
        let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;

        Ok(Self { pool })
    }

    /// Creates or replaces a cycle in the database
    pub async fn create_cycle(&self, cycle: Cycle) -> SqlxResult<()> {
        sqlx::query!(
            "
insert into cycles (
    id,
    block_number,
    creator,
    starting_block,
    block_length,
    vote_price,
    balance
)
values ($1, $2, $3, $4, $5, $6, $7)
on conflict (id) do update set
    block_number = $2,
    creator = $3,
    starting_block = $4,
    block_length = $5,
    vote_price = $6,
    balance = $7
            ",
            cycle.id as _,
            cycle.block_number as _,
            cycle.creator as _,
            cycle.starting_block as _,
            cycle.block_length as _,
            cycle.vote_price as _,
            cycle.balance as _
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Creates or replaces a vote in the database
    pub async fn create_vote(&self, vote: Vote) -> SqlxResult<()> {
        sqlx::query!(
            "
insert into votes (
    id,
    block_number,
    cycle_id,
    placer,
    symbol,
    amount,
    placement
)
values ($1, $2, $3, $4, $5, $6, $7)
on conflict (id) do update set
    block_number = $2,
    cycle_id = $3,
    placer = $4,
    symbol = $5,
    amount = $6,
    placement = $7
            ",
            vote.id as _,
            vote.block_number as _,
            vote.cycle_id as _,
            vote.placer as _,
            vote.symbol as _,
            vote.amount as _,
            vote.placement as _
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Sets the `claimed` field to true on a vote
    pub async fn claim_vote(&self, vote_id: BigDecimal) -> SqlxResult<()> {
        sqlx::query!(
            "
update votes
set claimed = true
where id = $1
            ",
            vote_id as _,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Sets the block height
    pub async fn set_block_height(&self, chain_id: BigDecimal, block_height: BigDecimal) -> SqlxResult<()> {
        sqlx::query!(
            "
insert into block_heights (chain_id, height)
values ($1, $2)
on conflict (chain_id) do update set height = $2
            ",
            chain_id as _,
            block_height as _
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_block_height(&self, chain_id: BigDecimal) -> SqlxResult<Option<BigDecimal>> {
        let result = sqlx::query!(
            "
select height
from block_heights
where chain_id = $1
            ",
            chain_id as _
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| r.height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers::{clients, images::postgres::Postgres, RunnableImage};

    /// Returns an available localhost port
    fn free_local_port() -> Option<u16> {
        let socket = std::net::SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, 0);
        std::net::TcpListener::bind(socket)
            .and_then(|listener| listener.local_addr())
            .map(|addr| addr.port())
            .ok()
    }

    /// Starts a new Postgres container and returns a `Database` instance
    async fn setup_db() -> Database {
        let port = free_local_port().expect("No ports free");
        let image = RunnableImage::from(Postgres::default())
            .with_tag("15-alpine")
            .with_mapped_port((port, 5432));
        let docker = clients::Cli::default();
        let _container = docker.run(image);
        let connection_string = format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port);
        let db = Database::new(&connection_string).await;
        assert!(db.is_ok());
        db.unwrap()
    }

    #[tokio::test]
    async fn create_new_database() {
        let port = free_local_port().expect("No ports free");
        let image = RunnableImage::from(Postgres::default())
            .with_tag("15-alpine")
            .with_mapped_port((port, 5432));
        let docker = clients::Cli::default();
        let _container = docker.run(image);
        let connection_string = format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port);
        let db = Database::new(&connection_string).await;
        assert!(db.is_ok());
    }
}
