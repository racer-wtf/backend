use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Error;

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(url: &str) -> Result<Self, Error> {
        let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;

        Ok(Self { pool })
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
