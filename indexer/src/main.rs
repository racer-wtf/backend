mod listener;

use std::env;

use database::Database;
use dotenvy::dotenv;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use listener::Listener;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // enable logging to console
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "indexer=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // create a database connection instance
    let database = Database::new(&env::var("DATABASE_URL").expect("DATABASE_URL is not set"))
        .await
        .expect("Connection failed for DATABASE_URL");

    // start indexing blockchain events
    Listener::new(database)
        .with_rpc_url(env::var("RPC_URL").expect("RPC_URL is not set"))
        .with_starting_block(
            env::var("START_HEIGHT")
                .expect("START_HEIGHT is not set")
                .parse()
                .expect("Invalid START_HEIGHT"),
        )
        .with_contract_address(env::var("RACER_ADDRESS").expect("RACER_ADDRESS is not set"))
        .start()
        .await
}
