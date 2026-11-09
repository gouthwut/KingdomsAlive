"""Open the modded test instance and verify the real server startup hook."""

import argparse
import base64
import json
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INSTANCE = "kingdoms-alive-m0"
WORLD = "KA-M0-Smoke"
MARKER = "[kingdoms_alive] M0 server_about_to_start dedicated=false"


def smoke(launcher: str) -> None:
    instances = json.loads(
        subprocess.check_output([launcher, "instance", "list", "--json"], timeout=20)
    )
    instance = next((entry for entry in instances if entry["id"] == INSTANCE), None)
    if not instance or not (instance["version"] or "").startswith("native-fdc87ae-"):
        raise RuntimeError(f"Build the mod in the {INSTANCE} instance first.")

    evidence = ROOT / "target" / "m0-smoke"
    evidence.mkdir(parents=True, exist_ok=True)
    with subprocess.Popen(
        [launcher, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        encoding="utf-8",
    ) as mcp:
        request_id = 0

        def request(method: str, params: dict, *, notification: bool = False) -> dict:
            nonlocal request_id
            request_id += 1
            message = {"jsonrpc": "2.0", "method": method, "params": params}
            if not notification:
                message["id"] = request_id
            mcp.stdin.write(json.dumps(message) + "\n")
            mcp.stdin.flush()
            if notification:
                return {}
            for line in mcp.stdout:
                response = json.loads(line)
                if response.get("id") == request_id:
                    if "error" in response:
                        raise RuntimeError(f"{method}: {response['error']}")
                    return response["result"]
            raise RuntimeError(f"{method}: Horizon MCP closed the connection.")

        def call(name: str, **arguments) -> dict:
            result = request("tools/call", {"name": name, "arguments": arguments})
            if result.get("isError"):
                raise RuntimeError(f"{name}: {result.get('content')}")
            return result

        started = False
        try:
            server = request("initialize", {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "kingdoms-alive-smoke", "version": "0.1.0"},
            })["serverInfo"]
            request("notifications/initialized", {}, notification=True)
            call("play", instance=INSTANCE, version=instance["version"],
                 world=WORLD, creative=True, offscreen=True)
            started = True
            ready = call("game_wait_ready", seconds=180)
            hello = call("game_hello")["structuredContent"]
            state = json.loads(ready["content"][0]["text"])
            if state["screen"] != "playing" or not state["server"]["joined"]:
                raise RuntimeError("The player did not enter the test world.")
            game_log = Path(instance["home"]) / "logs" / "latest.log"
            markers = [line for line in game_log.read_text(encoding="utf-8").splitlines()
                       if MARKER in line]
            if not markers:
                raise RuntimeError(f"Startup marker missing in {game_log}")
            report = {"instance": INSTANCE, "version": instance["version"], "world": WORLD,
                      "mcp": server, "hello": hello, "screen": state["screen"],
                      "dimension": state["dimension"], "markers": markers}
            (evidence / "result.json").write_text(json.dumps(report, indent=2) + "\n",
                                                   encoding="utf-8")
            print(markers[-1], flush=True)
            call("game_wait", ticks=80)
            screenshot = call("game_screenshot")
            png = next(item["data"] for item in screenshot["content"] if item["type"] == "image")
            (evidence / "world.png").write_bytes(base64.b64decode(png))
        finally:
            try:
                if started:
                    call("game_quit")
            finally:
                mcp.stdin.close()
                try:
                    mcp.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    mcp.terminate()
                    mcp.wait()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--launcher", help="Path to horizon-launcher.exe")
    args = parser.parse_args()
    if args.launcher is None:
        with (ROOT / ".codex" / "config.toml").open("rb") as config:
            args.launcher = tomllib.load(config)["mcp_servers"]["horizon"]["command"]
    smoke(args.launcher)
