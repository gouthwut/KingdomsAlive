# Contributing to Kingdoms Alive

Report reproducible problems with the bug report form and propose gameplay or tooling changes with the feature request form. Include the Minecraft, Horizon Nightly, and launcher versions. Remove credentials, local paths, and personal information from log excerpts. Keep game files, world saves, and generated packages out of commits and issue attachments.

Keep settlement rules in the Rust core and Horizon hooks in the adapter. Consult the [roadmap](docs/ROADMAP.md) before starting a new milestone. Verify changes against the pinned [Horizon API](docs/HORIZON_API.md), and test world changes in a disposable profile.

## Local checks

CI uses Rust 1.99.0 with rustfmt and Clippy, plus Python 3.12 for the official packager.

```powershell
cargo fmt --all --check
rustfmt --edition 2024 --check mixins/kingdoms_alive/server_start_smoke.rs
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

Follow the [testing guide](docs/TESTING.md) for in-game verification and the [release guide](docs/RELEASING.md) for packaging.
