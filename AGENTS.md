# Agent guidance

## Product boundary

`file-sharing` has exactly two top-level workflows:

1. share one file or folder once;
2. keep one folder synchronized with another person/device.

The ordinary filesystem is authoritative. Do not introduce a document library, search engine, JSON datastore, media catalog, or application-specific metadata model here.

## Architecture

- Keep reusable filesystem identity, manifests, verification, progress, and resume semantics separate from transports.
- Treat one-off transfer and Syncthing-backed persistent synchronization as adapters with different lifecycle semantics.
- Do not reimplement Syncthing synchronization.
- Do not reuse `multiplayer-setup-service` as an authority for human file sharing; reuse only genuinely generic low-level ideas when boundaries stay explicit.
- Prefer deterministic, portable representations. File content identity is based on bytes, not timestamps.
- Reject ambiguous filesystem behavior rather than silently weakening safety. Symlinks are currently unsupported.
- Keep networking authorization explicit. Discovery is never authorization.

## Validation

For Rust changes run:

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```
