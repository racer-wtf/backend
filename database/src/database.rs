use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::types::BigDecimal;
use sqlx::{Error, Postgres, Result as SqlxResult, Transaction};

use super::models::{Cycle, Leaderboard, Vote};

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(url: &str) -> Result<Self, Error> {
        let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;

        Ok(Self { pool })
    }

    pub async fn start_transaction(&self) -> SqlxResult<Transaction<'_, Postgres>> {
        self.pool.begin().await
    }

    /// Creates or replaces a cycle in the database
    pub async fn create_cycle(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        cycle: Cycle,
    ) -> SqlxResult<()> {
        sqlx::query!(
            "
insert into cycles (
    id,
    chain_id,
    block_number,
    creator,
    starting_block,
    block_length,
    vote_price,
    balance
)
values ($1, $2, $3, $4, $5, $6, $7, $8)
on conflict (id) do update set
    chain_id = $2,
    block_number = $3,
    creator = $4,
    starting_block = $5,
    block_length = $6,
    vote_price = $7,
    balance = $8
            ",
            cycle.id as _,
            cycle.chain_id as _,
            cycle.block_number as _,
            cycle.creator as _,
            cycle.starting_block as _,
            cycle.block_length as _,
            cycle.vote_price as _,
            cycle.balance as _,
        )
        .execute(&mut *tx)
        .await?;

        Ok(())
    }

    /// Deletes cycles greater than provided `from_block` from the database
    pub async fn delete_cycles(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        from_block: BigDecimal,
        chain_id: BigDecimal,
    ) -> SqlxResult<()> {
        sqlx::query!(
            "
delete from cycles
where
    starting_block >= $1
    and chain_id = $2
            ",
            from_block as _,
            chain_id as _
        )
        .execute(&mut *tx)
        .await?;

        Ok(())
    }

    /// Creates or replaces a vote in the database
    pub async fn create_vote(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        vote: Vote,
    ) -> SqlxResult<()> {
        sqlx::query!(
            "
insert into votes (
    id,
    chain_id,
    block_number,
    cycle_id,
    placer,
    symbol,
    amount
)
values ($1, $2, $3, $4, $5, $6, $7)
on conflict (id) do update set
    chain_id = $2,
    block_number = $3,
    cycle_id = $4,
    placer = $5,
    symbol = $6,
    amount = $7
            ",
            vote.id as _,
            vote.chain_id as _,
            vote.block_number as _,
            vote.cycle_id as _,
            vote.placer as _,
            vote.symbol as _,
            vote.amount as _,
        )
        .execute(&mut *tx)
        .await?;

        Ok(())
    }

    /// Deletes votes greater than provided `from_block` from the database
    pub async fn delete_votes(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        from_block: BigDecimal,
        chain_id: BigDecimal,
    ) -> SqlxResult<()> {
        sqlx::query!(
            "
delete from votes
where
    block_number >= $1
    and chain_id = $2
            ",
            from_block as _,
            chain_id as _
        )
        .execute(&mut *tx)
        .await?;

        Ok(())
    }

    /// Gets the count of votes for provided `cycle_id`
    pub async fn get_vote_count(
        &self,
        cycle_id: BigDecimal,
        chain_id: BigDecimal,
    ) -> SqlxResult<i64> {
        let result = sqlx::query!(
            "
select count(*)
from votes
where
    cycle_id = $1
    and chain_id = $2
            ",
            cycle_id as _,
            chain_id as _
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.count.unwrap_or(0))
    }

    /// Sets the `claimed` field to true on a vote
    pub async fn claim_vote(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        vote_id: BigDecimal,
        chain_id: BigDecimal,
    ) -> SqlxResult<()> {
        sqlx::query!(
            "
update votes
set claimed = true
where
    id = $1
    and chain_id = $2
            ",
            vote_id as _,
            chain_id as _
        )
        .execute(&mut *tx)
        .await?;

        Ok(())
    }

    /// Resets all claimed votes to false where the cycle is greater than or equal to the
    /// provided `from_block`
    pub async fn reset_vote_claims(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        from_block: BigDecimal,
        chain_id: BigDecimal,
    ) -> SqlxResult<()> {
        sqlx::query!(
            "
update votes
set claimed = false
where
    block_number >= $1
    and chain_id = $2
            ",
            from_block as _,
            chain_id as _
        )
        .execute(&mut *tx)
        .await?;

        Ok(())
    }

    /// Sets the block height
    pub async fn set_block_height(
        &self,
        chain_id: BigDecimal,
        block_height: BigDecimal,
    ) -> SqlxResult<()> {
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

    /// Gets the block height
    pub async fn get_block_height(&self, chain_id: BigDecimal) -> SqlxResult<BigDecimal> {
        let row = sqlx::query!(
            "
select height
from block_heights
where chain_id = $1
            ",
            chain_id as _
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.height).unwrap_or(0.into()))
    }

    /// Gets the current cycle from the database
    pub async fn get_current_cycle(&self, chain_id: BigDecimal) -> SqlxResult<Cycle> {
        sqlx::query_as!(
            Cycle,
            "
select *
from cycles
where
    current is true
    and chain_id = $1
order by block_number desc
limit 1
            ",
            chain_id as _,
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Gets leaderboard for the provided `cycle_id`
    pub async fn get_leaderboard(
        &self,
        cycle_id: BigDecimal,
        chain_id: BigDecimal,
    ) -> SqlxResult<Vec<Leaderboard>> {
        sqlx::query_as!(
            Leaderboard,
            "
select
	symbol,
	sum(amount) as amount,
	max(block_number) as max_block
from votes
where
    cycle_id = $1
    and chain_id = $2
group by symbol
order by
	amount desc,
	max_block asc
            ",
            cycle_id,
            chain_id
        )
        .fetch_all(&self.pool)
        .await
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
    #[allow(dead_code)]
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
