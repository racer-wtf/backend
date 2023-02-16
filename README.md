# Racer Backend

In this repository you'll find private worker processes that feed data to the frontend client.

- [`database`](database/) - defines shared methods for services to access indexed data
- [`indexer`](indexer/) - reads from blockchain nodes and organizes data for future use
- [`server`](server/) - both an http and websocket server for sending data to clients
