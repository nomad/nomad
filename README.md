# Nomad

[![CI][badge-ci]](https://github.com/nomad/nomad/actions/workflows/ci.yaml)
<!-- [![Discord][badge-discord]](https://discord.gg/xxxxxxxxxx) -->

Nomad brings real-time collaborative editing to Neovim. Create a session with
`:Mad collab start`, share the session ID with others, and they can join with
`:Mad collab join <session_id>`. Every peer in the session sees live cursor
positions, selections, and text edits as they happen — all [powered][crdt-cola]
[by][crdt-puff] [custom][crdt-pando] [CRDTs][crdt-wiki] that keep everything in
sync.

But real-time editing is just the first step. The longer-term goal is
collaborative coding that works across different editors, across time (both
synchronous and asynchronous collaboration), and across different types of
contributors — human and machine alike. Whether you're pair programming with a
colleague in real-time, reviewing changes someone made last night, or working
alongside agents, Nomad aims to make it seamless.

Right now, it works in Neovim. Eventually, it'll work everywhere.

---

## Getting Started

- [Installation][docs-installation]
- [Usage][docs-usage]
- [Configuration][docs-configuration]
- [Build from source][docs-building]

[badge-ci]: https://github.com/nomad/nomad/actions/workflows/ci.yaml/badge.svg
[badge-discord]: https://img.shields.io/discord/xxxxxxxxxxxxxxxxxxx
[crdt-cola]: https://github.com/nomad/cola
[crdt-pando]: https://github.com/nomad/pando
[crdt-puff]: https://github.com/nomad/puff
[crdt-wiki]: https://en.wikipedia.org/wiki/Conflict-free_replicated_data_type
[docs-building]: https://github.com/nomad/nomad/tree/main/docs/building.md
[docs-configuration]: https://github.com/nomad/nomad/tree/main/docs/configuration.md
[docs-installation]: https://github.com/nomad/nomad/tree/main/docs/installation.md
[docs-usage]: https://github.com/nomad/nomad/tree/main/docs/usage.md
