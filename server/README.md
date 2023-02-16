# Racer Server

This process spawns an HTTP server and websocket server.

## Running

`cargo run --release`

## Websocket server

The websocket server is built with a publish-subscribe architecture. You can send messages to the server that define what you'd like to subscribe to.

```json
{
  "address": "0x0000000000000000000000000000000000000000",
  "subscriptions": [
    "online",
    "leaderboard"
  ]
}
```

```
address - any ethereum address prefixed with 0x
```

```
subscriptions - a list of any subscriptions (see below)
```

### Subscriptions

#### **`online` - subscribes to online count**

Example response:
```json
{
  "count": 23,
  "type": "online"
}
```

Frequency: **Every 5 seconds**

#### **`leaderboard` - subscribes to leaderboard stats**

Example response:
```json

```

Frequency: **Every new block mined**

### Example

```bash
$ websocat ws://127.0.0.1:3000/ws
> {"subscriptions": ["online"]}
{"count":1,"type":"online"}
{"count":1,"type":"online"}
{"count":1,"type":"online"}
...
```
