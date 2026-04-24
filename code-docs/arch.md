# Architecture

```mermaid
graph TD
    User["User"] -->|interacts via| TUI["TUI Client"]
    TUI -->|HTTP routes| Server["Server"]
    Server -->|drives| GameLoop["Game Loop"]
    GameLoop -->|reads/writes| DB["Database"]
    GameLoop -->|processes| Updates["Entity, Engagement & Location Updates"]
    Server -->|places interactions into| Mailboxes["Entity Mailboxes"]
    Mailboxes -->|resolved by| GameLoop
    AI["AI Agents"] -->|interact with| Mailboxes
    AI -->|stream response to| Player["Player"]
```

- Users interact with a TUI client.
- The server exposes routes for all major functionality to the client.
- Behind the server is the game loop.
- Within the game loop, game state is fetched from the DB, then entity, engagement, and location updates are processed.
- The server can place interactions into entity mailboxes, which are resolved as part of the game loop.
- AI agents interact with mailboxes.
    - They can stream responses into a mailbox and directly to the player.