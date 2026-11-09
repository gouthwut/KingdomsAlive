---
name: horizon-native
description: Build and verify Kingdoms Alive native Rust mods against the pinned Minecraft Horizon hook catalog. Use for Horizon mixins, server-side helpers, native-build, .hnmod packages, and in-game smoke tests.
---

# Horizon native mods

- Read [the current handoff](../../../docs/HANDOFF.md) and [verified API notes](../../../docs/HORIZON_API.md). Use the revision from `compatibility.json`.
- Verify target and bindings in that revision's `native-hooks.json` and corresponding Rust source. Prefer documented catalog hooks and thin adapters.
- Build with the Horizon launcher in a separate instance. Confirm runtime behavior in a disposable world using the [test checklist](../../../docs/TESTING.md). Treat format checks, compilation, and runtime proof separately.
- For updates, check [bundle documentation](https://playhorizon.dev/docs/mod-manifest), [hook documentation](https://playhorizon.dev/docs/mod-code), and [game automation](https://playhorizon.dev/docs/agents).
