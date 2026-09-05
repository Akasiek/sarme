# Sarmë

A self-hostable lyrics downloader for your music library.

## Name definition

sarmë 0
Q. writing

> sarmë noun "writing" (VT39:8). Cf. sarat.

[Source - Parf Edhellen](https://www.elfdict.com/wt/113657)


## Development

### Prerequisites

- [Rust](https://rustup.rs) (stable)
- [systemfd](https://github.com/mitsuhiko/systemfd) (if you want to use the hot-reload server)
- [pnpm](https://pnpm.io)

### Setup

#### `.env` file

Configuration is read from environment variables. For local development, an
optional `.env` file in the project root is loaded first:

```env
LIBRARY_DIR=/absolute/path/to/music

# Optional values and their defaults:
DATA_DIR=./data
DATABASE_URL=sqlite://data/sarme.db
HOST=0.0.0.0
PORT=8080
SCAN_INTERVAL_SECONDS=3600
```

`LIBRARY_DIR` is required and must point to an existing directory using an
absolute path. `HOST` must be an IP address, `PORT` must fit in the TCP port
range, and `SCAN_INTERVAL_SECONDS` must be greater than zero. Invalid values
prevent the application from starting and produce a descriptive error.

#### Install frontend dependencies

```bash
# Install JS dependencies
cd templates && pnpm install && cd ..

```

### Running the server

With hot-reload (restarts server on code changes):

```bash
systemfd --no-pid -s http::8080 -- cargo watch -x run
```

or without hot-reload:

```bash
cargo run
```

### Tailwind CSS watch

In a separate terminal from the `templates/` directory:

```bash
cd templates && pnpm run watch:css
```

Watches `styles/styles.css` and rebuilds `styles/tailwind.css` on every change.
