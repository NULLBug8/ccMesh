# ccMesh

ccMesh is a Web console plus Rust HTTP backend for an AI proxy gateway. It provides endpoint management, model mapping, ordered routing, circuit breaker/degradation rules, balance checks, request logs, statistics, and OpenAI / Claude / Codex forwarding.

## Development

```bash
pnpm install
pnpm dev
```

The frontend dev server defaults to `http://127.0.0.1:5173`.

## Backend

```bash
pnpm server:dev
```

Environment variables:

- `CCMESH_PORT`: backend HTTP port, for example `3001`.
- `CCMESH_DATA_DIR`: data directory. If unset, ccMesh uses the system app data directory under `ccmesh`.

Open `http://127.0.0.1:3001/` after the backend starts.

## Build

```bash
pnpm build
pnpm server:build
```

The backend binary is written to `src-tauri/target/release/`; Web assets are written to `dist/`.

## Verification

```bash
pnpm test
pnpm check:front
pnpm check:rust
cargo test --manifest-path src-tauri/Cargo.toml
```