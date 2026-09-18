# Roadmap

The roadmap stays deliberately small. Product behavior should emerge from the two user workflows rather than from a generic file-management abstraction.

## A. Shared transfer foundation

- [x] Define deterministic manifests for a shared file or folder.
- [x] Use SHA-256 for exact file integrity and omit timestamps from content identity.
- [x] Preserve empty directories and stable relative paths.
- [x] Reject symbolic links until their cross-platform and security semantics are explicit.
- [ ] Add receiver-side manifest validation and safe destination-path construction.
- [ ] Add resumable chunk descriptions without changing whole-file identity.
- [ ] Add explicit transfer progress and cancellation contracts.

## B. Share once

- [ ] Start a temporary local send session for one file or folder.
- [ ] Pair a receiver explicitly; do not require an account.
- [ ] Transfer and verify bytes against the accepted manifest.
- [ ] Resume interrupted transfers.
- [ ] Add LAN discovery as a convenience, never as authorization.
- [ ] Add QR/code-based pairing for cross-device handoff.
- [ ] Decide when relay/NAT traversal is justified by actual usage.

## C. Keep synced

- [ ] Detect/configure a local Syncthing installation through a narrow adapter.
- [ ] Share an existing folder without moving it into application-owned storage.
- [ ] Surface peer, folder, sync, pause, and conflict state.
- [ ] Keep Syncthing configuration/protocol semantics behind the adapter.
- [ ] Support removing the sharing relationship without deleting the user's folder.

## Explicit non-goals

- Document indexing or search.
- Household/document-domain metadata.
- JSON-database APIs.
- Game-specific asset distribution.
- A new synchronization protocol that duplicates Syncthing.
- Application-owned opaque storage replacing ordinary files.
