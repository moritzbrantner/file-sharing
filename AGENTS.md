# Repository agent guidance

Shared engineering policy belongs in the managed `coding-agent-conventions` snapshot. Reusable agent procedures belong in `coding-agent-skills`. Deterministic validation, evidence, convergence, and integration mechanics belong in `coding-tooling`. Keep this file focused on `file-sharing` boundaries.

## Product boundary

`file-sharing` has exactly two top-level workflows:

1. share one file or folder once;
2. keep one folder synchronized with another person/device.

The ordinary filesystem is authoritative. Do not introduce a document library, search engine, JSON datastore, media catalog, or application-specific metadata model here.

## Authority boundaries

- Owns: `file-sharing/filesystem-share-manifests`, `file-sharing/one-off-transfer-semantics`, `file-sharing/receive-safety`, `file-sharing/syncthing-adapter`.
- Adapts: `syncthing/persistent-folder-sync`, `coding-agent-conventions/policy`, `coding-agent-skills/procedures`.
- Non-authoritative: document indexing/search, household document metadata, generic JSON APIs, game-specific asset distribution, and Syncthing protocol semantics.
- Prohibited write-back: transport, discovery, progress, and sync-state adapters must not silently reinterpret application-owned metadata or replace ordinary filesystem ownership.

## Architecture

- Keep reusable filesystem identity, manifests, verification, progress, and resume semantics separate from transports.
- Treat one-off transfer and Syncthing-backed persistent synchronization as adapters with different lifecycle semantics.
- Do not reimplement Syncthing synchronization.
- Do not reuse `multiplayer-setup-service` as an authority for human file sharing; reuse only genuinely generic low-level ideas when boundaries stay explicit.
- Prefer deterministic, portable representations. File content identity is based on bytes, not timestamps.
- Reject ambiguous filesystem behavior rather than silently weakening safety. Symlinks are currently unsupported.
- Keep networking authorization explicit. Discovery is never authorization.

## Agent workflow

- On a fresh machine or after the declared environment contract changes, run `bash scripts/codex-environment.sh setup`. Use `maintenance` when dependency state changes.
- Use the narrowest relevant Rust check during implementation.
- Before handoff, run the repository-owned `coding-tooling` fast tier. Hosted workflows are adapters over that semantic contract rather than a second validation definition.
- Treat exact-head hosted evidence as integration evidence. Do not merge because a stale run or a different revision was green.
- Use `coding-tooling converge` for convergence work; do not hide an active finding by weakening the check or silently skipping a suggested test.

## Validation

The fast semantic gate is declared in `.coding-tooling.json`. Repository-owned Rust capabilities remain the source of truth for format, lint, build, and unit-test execution.
