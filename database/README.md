# Database

## Migrations

```bash
sqlx migrate add -r <name>   # add a migration
sqlx migrate run             # runs all pending migrations
sqlx migrate revert          # reverts most recent migration
```
