# file-sharing

Local-first file and folder sharing with two deliberately small product modes:

- **Share once** — send one file or folder to another person or device, verify what arrived, then end the relationship.
- **Keep synced** — share a folder continuously by integrating an existing synchronization engine such as Syncthing.

The ordinary filesystem remains authoritative. `file-sharing` moves and synchronizes bytes; it does not interpret documents, index their contents, expose JSON data as an API, or own application-specific metadata.

## Architecture

The reusable core owns portable filesystem manifests, stable relative paths, content hashes, verification, progress, and resume semantics.

One-off transfer and persistent synchronization are separate adapters over that core:

- one-off transfer owns temporary send/receive sessions;
- persistent sync delegates replication to Syncthing rather than reimplementing its synchronization protocol.

## Current slice: deterministic manifests

The first implemented primitive describes exactly what is being shared before networking begins:

- files are identified by byte length and SHA-256;
- folder entries use stable relative paths and deterministic ordering;
- empty directories are retained;
- timestamps are deliberately excluded from content identity;
- symbolic links are rejected until their cross-platform and security semantics are explicit.

Generate a manifest with:

```sh
cargo run -- manifest ./path/to/file-or-folder
```

The resulting JSON is intended to become the acceptance and verification boundary for one-off transfers and a reusable observation surface for sync adapters.

See [ROADMAP.md](./ROADMAP.md) for the next slices.
