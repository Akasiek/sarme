# Sarmë agent guide

## Product and scope

Sarmë is a self-hosted application that finds synchronized lyrics for a local
music library and writes them as sidecar `.lrc` files. The first release is for
one administrator and one mounted music directory.

The MVP flow is:

`scan -> read metadata -> LRCLIB lookup -> score match -> write .lrc`

Supported audio formats are FLAC, MP3, Opus/Ogg, and M4A. A filesystem watcher
is not part of the MVP; scheduled incremental scans are the reliable baseline,
including for NFS and bind-mounted libraries.

## Established architecture

- Use Rust and Axum for the HTTP server and application services.
- Use SQLite with explicit, versioned migrations for persistent state.
- Keep long-running scan and lookup work in a controlled worker/queue; do not
  block request handlers.
- Render the web panel on the server with Askama templates and enhance it with
  HTMX. Axum serves full HTML pages, HTML fragments, and static assets.
- Organize templates as `templates/layouts/`, `templates/pages/`, and
  `templates/components/`.
- Serve HTMX locally with the application. Do not depend on a third-party CDN
  for it.
- Keep the server as the source of truth. Use HTMX fragments, forms, and
  polling for interactive updates; ordinary form submissions must remain
  usable.
- There is no separate frontend process or JSON API client for the panel.

`/home/akasiek/Code/Rust/laneya` is the structural reference for Askama + HTMX:
inspect its template layout, Axum routes, and HTML-fragment handlers before
introducing the web layer. Reuse its mechanics where they fit Sarmë; do not
copy unrelated product behavior.

## Music-library safety rules

- Treat the mounted music directory as user data.
- Do not modify audio files in the MVP.
- Write `track-name.lrc` beside `track-name.ext`; never write outside the
  configured music root.
- Do not overwrite an existing `.lrc` file automatically.
- Write a new `.lrc` atomically through a temporary file and rename.
- Preserve UTF-8 and Unicode paths. Surface permission and per-track errors in
  persistent status; one bad file must not abort a scan.
- Accept automatic matches only when deterministic scoring is unambiguous.
  Keep uncertain candidates for manual review. Record no-result attempts so
  they can be retried later with a controlled policy.

## Engineering workflow

- This repository is intentionally minimal. Inspect `Cargo.toml` and the
  current tree before adding dependencies, commands, or project structure.
- Prefer small, cohesive changes that correspond to one backlog card.
- Keep configuration explicit and validated at startup: music root, data and
  database paths, listen address/port, and scan interval.
- Keep schema migrations idempotent and test an application restart before
  claiming persistence works.
- When Rust code exists, run the narrowest relevant checks first; normally
  `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- Do not introduce a separate SPA build pipeline. Template compilation and
  route/fragment tests belong in the Rust workflow.
