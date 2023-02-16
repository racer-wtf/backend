# Racer Backend

In this repository you'll find private worker processes that feed data to the frontend client.

- [`database`](database/) - defines shared methods for services to access indexed data
- [`indexer`](indexer/) - reads from blockchain nodes and organizes data for future use
- [`server`](server/) - both an http and websocket server for sending data to clients

## Local Environment

Requirements:

- [`Docker`](https://www.docker.com/products/docker-desktop/) 
- [`sqlx`](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)

```bash
make db-create   # creates database
make db-migrate  # runs all pending database migrations
make db-revert   # reverts the most previous database migration
make db-show     # shows connection string
make db-drop     # stops and removes the database docker container
make db-reset    # alias for `db-drop`, `db-create`, `db-migrate` sequentially
```
