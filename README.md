# Kingdoms Alive

Kingdoms Alive is a native Rust mod for Minecraft Java Edition on Horizon. Its goal is to give villages an economy of their own. Villagers will gather Minecraft resources, spend them from settlement stockpiles, and build within approved territory.

## Development status

- **M0 complete.** Horizon compiled the native hook, and a separate Windows test profile opened a disposable world and confirmed that the integrated server called it.
- **M1 complete.** The server can register one existing village in an approved territory, calculate its priority on the server tick and restore its versioned snapshot when the world reopens. These behaviors were verified in an isolated game world.
- **M2 pilot.** In a separate test world, an existing villager walked to an approved chest and deposited one oak log from its off hand. The chest respected capacity, and saving and reopening preserved one item without replaying the delivery. See the [M2 test setup](docs/TESTING.md#пилот-m2).
- **M2 warehouse registry.** Approved single chests now supply observed stock for settlement planning; historical M1 counters are ignored. Up to eight chests are scanned per step. Missing or conflicting registered containers pause planning instead of reusing stale counts. See the [warehouse setup](docs/TESTING.md#реестр-склада-m2b).
- **Pilot safety limit.** Delivery is explicitly enabled per test world, never generates cargo, and refuses replay after a claim. It is not crash-atomic: a crash between separate entity and chest writes can lose the log. Do not enable it in valued worlds.
- **Experimental lumberjack — currently disabled.** A previous build verified two real harvests and deliveries in a disposable world. New worker mutations are now disabled while crash-safe ordering is unresolved, even with test-world approval. Diagnostic reconciliation and immutable actual-loot receipts do not make the separate world writes atomic. See the [current handoff](docs/HANDOFF.md).
- Crash-safe recovery, general-purpose jobs, farming and construction remain unfinished. Dedicated servers and multiplayer are unverified.

See the [roadmap](docs/ROADMAP.md) and [progress log](docs/PROGRESS.md) for milestone criteria and test results.

## Compatibility

| Component | Verified target |
| --- | --- |
| Minecraft Java Edition | 26.3. Use your own copy. |
| Horizon | Nightly `nightly-2026-11-08-fdc87ae` |
| Horizon Launcher | 0.2.57 on Windows |

The current manifest pins a Horizon revision. Use the matching Nightly in a separate profile. See [compatibility.json](compatibility.json) for the exact revisions.

## Downloads

Approved builds will appear as `.hnmod` files on [GitHub Releases](https://github.com/gouthwut/KingdomsAlive/releases). Each package contains the native mod sources and metadata. Horizon Launcher compiles the package for your profile.

## Installation

1. Install Horizon Launcher and prepare Minecraft Java Edition 26.3 using your copy of the game. [Horizon installation guide](https://playhorizon.dev/docs/install)
2. Download a `.hnmod` from Releases and extract it to a folder. Use the folder containing `horizon-native.json` as the bundle path.
3. In PowerShell, set the bundle path and build into a separate test profile. The trust flag grants the native code access to run as part of Horizon, so use a bundle you trust.

   ```powershell
   $launcher = "$env:LOCALAPPDATA/Programs/Horizon Launcher/horizon-launcher.exe"
   $bundle = (Resolve-Path ./kingdoms_alive-0.1.0).Path
   & $launcher instance create 'Kingdoms Alive M1' # Once per profile
   & $launcher native-build $bundle --instance kingdoms-alive-m1 --version nightly-2026-11-08-fdc87ae --profile performance --trust-native-code
   & $launcher play --instance kingdoms-alive-m1 --world KA-M1-Village --creative
   ```

4. Follow the [M1 testing guide](docs/TESTING.md) to register a village and check the resulting snapshot.

For a source checkout, point `$bundle` at `./mixins/kingdoms_alive`. To play without the mod, select your regular Horizon profile.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for local checks, packaging, and the release process.

Kingdoms Alive builds on [Horizon](https://github.com/horizon-rust/horizon) and uses its native packaging tools. Codex assisted with development and testing.
