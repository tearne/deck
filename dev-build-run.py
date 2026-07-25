#!/usr/bin/env -S uv run --script --
# /// script
# requires-python = "==3.12.*"
# dependencies = ["rich"]
# ///

import argparse
import os
import shlex
import subprocess
import sys
from pathlib import Path

from rich.console import Console

VERSION = "1.0.4"
PROJECT_NAME = "deck"

_console = Console()


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Build the project in an incus container, pull the release binary, and run it locally "
                     "with an isolated config."
    )
    parser.add_argument("container", help="name of the incus container to build in and pull from")
    parser.add_argument("app_args", nargs=argparse.REMAINDER, help="arguments forwarded to the launched app")
    parser.add_argument("--version", action="version", version=f"%(prog)s {VERSION}")
    args = parser.parse_args()

    remote_path = discover_remote_path(args.container, PROJECT_NAME)
    build_in_container(args.container, remote_path)
    binary = pull_binary(args.container, remote_path, PROJECT_NAME)
    discard_local_config()
    launch(binary, args.app_args)


def discover_remote_path(container: str, project_name: str) -> str:
    cmd = (
        f"incus exec {container} -- bash -lc "
        f"'d=~/{project_name}; if [ -d \"$d/.git\" ]; then echo \"$d\"; "
        f"else find / -maxdepth 4 -type d -name {project_name} 2>/dev/null | head -1; fi'"
    )
    result = run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        sys.exit(result.returncode)
    path = result.stdout.strip()
    if not path:
        _console.print(f"Could not find project '{project_name}' in container '{container}'.", style="bold red")
        sys.exit(1)
    return path


def build_in_container(container: str, remote_path: str) -> None:
    # incus exec runs as root by default, but the discovered checkout belongs to
    # whichever user's home it lives under (and that's who has cargo installed).
    build_cmd = f"cd {remote_path} && cargo build --release"
    user = home_dir_owner(remote_path)
    if user:
        cmd = f"incus exec {container} -- su - {user} -c {shlex.quote(build_cmd)}"
    else:
        cmd = (
            f"incus exec {container} -- bash -lc "
            + shlex.quote(f"[ -f ~/.cargo/env ] && source ~/.cargo/env; {build_cmd}")
        )
    result = run(cmd)
    if result.returncode != 0:
        sys.exit(result.returncode)


def home_dir_owner(path: str) -> str | None:
    parts = Path(path).parts
    return parts[2] if len(parts) >= 3 and parts[1] == "home" else None


def pull_binary(container: str, remote_path: str, project_name: str) -> Path:
    local_binary = Path(project_name)
    result = run(f"incus file pull {container}{remote_path}/target/release/{project_name} ./{project_name}")
    if result.returncode != 0:
        sys.exit(result.returncode)
    local_binary.chmod(0o755)
    return local_binary


def discard_local_config() -> None:
    # A stale config.toml silently overlays old keybindings onto a new build's
    # defaults; the launched binary recreates the file from its embedded config.
    Path("config.toml").unlink(missing_ok=True)


def launch(binary: Path, app_args: list[str]) -> None:
    cmd = " ".join([f"./{binary.name}", "--local-config", *app_args])
    run(cmd)


def run(cmd: str, **kwargs) -> subprocess.CompletedProcess:
    _console.print(f"$ {cmd}", style="cyan")
    return subprocess.run(f"set -o pipefail && {cmd}", shell=True, executable="/bin/bash", **kwargs)


if __name__ == "__main__":
    if not os.environ.get("VIRTUAL_ENV"):
        print("Error: no virtual environment detected. Run this script via './dev-build-run.py' (requires uv), or activate a virtual environment first.")
        sys.exit(100)
    main()
