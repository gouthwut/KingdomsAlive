# Packaging and releases

## What CI verifies

The [workflow](../.github/workflows/ci-release.yml) checks formatting for both the Rust core and the native hook, runs Clippy and core tests, then packages and validates the mod on a GitHub-hosted Windows runner. It uses the [official packager at a fixed revision](https://github.com/horizon-rust/horizon-mods/blob/e2e833d68c3d18ea75af537e73a3bd2bd2bd21fb/native/tools/native.py) and builds the package checker from the [pinned Horizon revision](https://github.com/horizon-rust/horizon/tree/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/crates/cargo-horizon-mod).

The packager reads the compatibility file, the native bundle, and the project's LICENSE when present. The current bundle contains the M0 hook. The separate M1 Rust library is checked by CI and is outside the bundle.

Packaging and validation require Python, Rust and access to public source and dependency repositories. Compiling the mod into the launcher and starting a Minecraft world are separate checks. Perform them with your own Minecraft files on an isolated local profile and disposable world. See the [testing guide](TESTING.md).

## Enable releases

1. After the owner approves public source publication, create the empty public `gouthwut/KingdomsAlive` repository and push `main` with its existing history. Add a remote only if its name is unused. Keep existing remotes intact.
2. Verify the hosted CI run on `main`. Leave the `RELEASES_ENABLED` Actions variable unset until the release controls below are configured and approved.
3. Create the `release` environment in Settings → Environments. Set a required reviewer, disable administrator bypass, and allow only tags matching `v*` in the deployment policy. Then set the repository Actions variable `RELEASES_ENABLED` to `true`. [GitHub environment documentation](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments)
4. Commit the approved LICENSE, ensure the mod manifest version matches the planned tag, and record the in-game test result for that revision. After explicit release approval, push only the intended `v<version>` tag from the verified commit.
5. Review the pending release deployment after the checks pass. The job publishes the checked `.hnmod` to the existing tag. Branch, pull request and manual CI runs run checks without uploading an artifact. [GitHub CLI release documentation](https://cli.github.com/manual/gh_release_create)

## Manual fallback

If hosted packaging is unavailable, check out the two upstream revisions from the workflow in separate directories. Run these commands from the Kingdoms Alive root using Python 3.12 and Rust 1.99.0. Set the two checkout paths for your machine.

```powershell
$mods = (Resolve-Path ../horizon-mods).Path
$horizon = (Resolve-Path ../horizon).Path
$modRoot = (Get-Location).Path
cargo +1.99.0 build --locked -p cargo-horizon-mod --manifest-path "$horizon/Cargo.toml" --target-dir "$horizon/target"
if ($LASTEXITCODE -ne 0) { throw 'Horizon package checker build failed.' }
$result = python "$mods/native/tools/native.py" package $modRoot
if ($LASTEXITCODE -ne 0) { throw 'Packaging failed.' }
$package = $result | ConvertFrom-Json
& "$horizon/target/debug/cargo-horizon-mod.exe" check $package.file --json
if ($LASTEXITCODE -ne 0) { throw 'Package validation failed.' }
Get-FileHash -LiteralPath $package.file -Algorithm SHA256
```

Use that exact package for the smoke test. After explicit release approval, upload the verified package to the release for the matching tag. Keep local game files, logs, screenshots, and build output out of the release.
