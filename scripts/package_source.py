#!/usr/bin/env python3
"""Bundle the checked-out source and locked Rust dependencies beside binaries."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import zipfile


def capture_utf8(arguments: list[str], cwd: Path | None = None) -> str:
    # Git/Cargo emit UTF-8 to pipes, even when Windows Python defaults to cp1252.
    return subprocess.check_output(arguments, cwd=cwd, encoding="utf-8")


def package_source(output: Path) -> Path:
    output = output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    revision = capture_utf8(["git", "rev-parse", "HEAD"]).strip()
    with tempfile.TemporaryDirectory(prefix="hoshi-source-") as temporary:
        staging = Path(temporary)
        tree = staging / "hoshi-paint-source"
        tree.mkdir()
        archive = staging / "tracked.zip"
        subprocess.run(["git", "archive", "--format=zip", "--output", str(archive), revision], check=True)
        with zipfile.ZipFile(archive) as source:
            source.extractall(tree)
        vendored = capture_utf8(
            ["cargo", "vendor", "--locked", "--versioned-dirs", "vendor"], cwd=tree,
        )
        config = tree / ".cargo" / "config.toml"
        config.parent.mkdir(exist_ok=True)
        with config.open("a", encoding="utf-8") as handle:
            handle.write("\n" + vendored)
        metadata = json.loads(capture_utf8(
            ["cargo", "metadata", "--offline", "--locked", "--format-version", "1"],
            cwd=tree,
        ))
        vendor = (tree / "vendor").resolve()
        for package in metadata["packages"]:
            if package["source"] is not None and not Path(package["manifest_path"]).resolve().is_relative_to(vendor):
                raise RuntimeError(f"Dependency is missing from source bundle: {package['name']}")
            if package["id"] in metadata["workspace_members"] and package["license"] != "GPL-3.0-only":
                raise RuntimeError(f"Workspace license mismatch: {package['name']}")
        (tree / "SOURCE_REVISION").write_text(revision + "\n", encoding="utf-8")
        (tree / "BUILD_FROM_SOURCE.md").write_text(
            "# Build this source archive\n\n"
            "This archive contains the application source and locked Rust dependencies, "
            "including their upstream license files.\n\n"
            "Install the Rust version in rust-toolchain.toml and the system build "
            "prerequisites described in README.md and spikes/desktop-canvas/README.md. "
            "Then run from this directory:\n\n"
            "```sh\ncargo build --offline --locked -p hoshi-canvas-spike\n```\n\n"
            "The .cargo/config.toml source replacement uses the included vendor directory. "
            "System libraries and the Rust toolchain are installed separately. "
            "SOURCE_REVISION identifies the binary's source commit.\n",
            encoding="utf-8",
        )
        built = Path(shutil.make_archive(str(output.with_suffix("").with_suffix("")), "gztar", staging, tree.name))
        if built != output:
            shutil.move(built, output)
    return output


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    try:
        print(package_source(parser.parse_args().output))
    except Exception as error:
        if os.environ.get("GITHUB_ACTIONS") == "true":
            message = f"{type(error).__name__}: {error}".replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
            print(f"::error title=Source packaging failed::{message}")
        raise
