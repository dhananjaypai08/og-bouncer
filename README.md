# og-bouncer

A PostgreSQL protocol proxy/bouncer written in Rust. It intercepts and forwards PostgreSQL wire protocol messages between clients and a PostgreSQL server, with the ability to parse, inspect, and track authentication state.

## Running the Project

### 1. Start PostgreSQL

```bash
docker compose up -d
```

This starts a PostgreSQL 16 instance on `localhost:5432` with credentials:
- **User**: `test`
- **Password**: `test`
- **Database**: `test`

### 2. Start the Bouncer

```bash
cd rustbouncer
cargo run
```

The bouncer listens on `0.0.0.0:6432` and proxies connections to `127.0.0.1:5432`.

### 3. Test the Connection

**Test direct connection to PostgreSQL (server):**
```bash
psql -h localhost -p 5432 -U test -d test
```

**Test connection through the bouncer (client):**
```bash
psql -h localhost -p 6432 -U test -d test
```

When connecting through the bouncer, you'll see protocol parsing and authentication state logs in the bouncer terminal.
