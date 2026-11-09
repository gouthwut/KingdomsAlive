# Kingdoms Alive

Kingdoms Alive is a native Rust mod for Minecraft Java Edition on Horizon. Its goal is to give villages an economy of their own. Villagers will gather Minecraft resources, spend them from settlement stockpiles, and build within approved territory.

## Development status

- **M0 complete.** Horizon compiled the native hook, and a separate Windows test profile opened a disposable world and confirmed that the integrated server called it.
- **M1 complete.** The server can register one existing village in an approved territory, calculate its priority on the server tick and restore its versioned snapshot when the world reopens. These behaviors were verified in an isolated game world.
- Resource gathering, hauling, and construction are planned features. Dedicated servers and multiplayer are unverified.

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
