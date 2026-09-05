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

Create a `.env` file in the project root with the following content:

```env
DATABASE_URL=./database.db
```

#### Install dependencies and apply database migrations

```bash
# Install JS dependencies
cd templates && pnpm install && cd ..

# Apply database migrations
diesel migration run
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

### Database migrations

```bash
# Apply pending migrations
diesel migration run

# Revert the last migration
diesel migration revert
```
