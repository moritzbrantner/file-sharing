# file-sharing

Local-first file and folder sharing with two deliberately small product modes:

- **Share once** — send one file or folder to another person or device, verify what arrived, then end the relationship.
- **Keep synced** — share a folder continuously by integrating an existing synchronization engine such as Syncthing.

The ordinary filesystem remains authoritative. `file-sharing` moves and synchronizes bytes; it does not interpret documents, index their contents, expose JSON data as an API, or own application-specific metadata.

## Architecture direction

The reusable core should cover filesystem manifests, stable relative paths, content hashes, transfer progress, resumability, integrity verification, peer/device identity, and explicit acceptance.

One-off transfer and persistent sync are separate adapters over that core:

- one-off transfer owns temporary send/receive sessions;
- persistent sync delegates replication to Syncthing rather than reimplementing its synchronization protocol.

## First slice

The first implementation slice builds deterministic manifests for a file or folder. A manifest is the boundary used by later transfer code to describe exactly what is being shared and verify received bytes.

The initial CLI will expose that primitive before networking is added, so the transfer protocol can be built on a tested, deterministic representation rather than inventing file identity inside the transport layer.
