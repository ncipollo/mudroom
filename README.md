# mudroom

A TUI client and server for multiplayer text adventure games (MUDs).

## Installation

```sh
cargo install mudroom
```

## Usage

### Server

Start a server with an optional name:

```sh
mudroom server
mudroom server --name my-server
```

### Client

Connect directly to a server URL:

```sh
mudroom client --url http://localhost:3000
```

Or launch the discovery UI to find servers on your local network:

```sh
mudroom client
mudroom
```

## License

MIT
