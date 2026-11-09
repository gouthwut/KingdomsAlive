"""Test the actual adapter against a local checkout of pinned Horizon, without a game.
Usage: python tools/check_loot_codec.py PATH_TO_PINNED_HORIZON
The caller supplies revision fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca; no downloads or world writes.
"""
import json
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
horizon = Path(sys.argv[1]).resolve()
for crate in ('horizon-items', 'horizon-world'):
    if not (horizon / 'crates' / crate / 'Cargo.toml').is_file():
        raise SystemExit('Pinned Horizon checkout required')
harness = root / 'target' / 'recovery-codec-check'
(harness / 'src').mkdir(parents=True, exist_ok=True)
quote = lambda p: json.dumps(str(p).replace('\\', '/'))
manifest = '[package]\nname="ka-recovery-codec-check"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[dependencies]\n'
for crate in ('horizon-items', 'horizon-world'):
    manifest += f'{crate} = {{ path = {quote(horizon / "crates" / crate)} }}\n'
manifest += 'serde_json="1"\n'
(harness / 'Cargo.toml').write_text(manifest, encoding='utf-8')
(harness / 'src/lib.rs').write_text('#[path = "../../../mixins/kingdoms_alive/loot_codec.rs"]\nmod loot_codec;\n#[allow(dead_code)]\nfn exercise(stacks: &[horizon_items::items::ItemStack]) -> std::io::Result<String> { loot_codec::serialize_loot(stacks) }\n', encoding='utf-8')
subprocess.run(['cargo', 'test', '--manifest-path', str(harness/'Cargo.toml')], cwd=root, check=True)
