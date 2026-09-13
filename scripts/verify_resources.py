"""Hash read-only inputs before/after the Rust audit; emit metadata only to stdout."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def inventory(root):
    paths = [root / "bin" / name for name in (
        "GraphicInfo_66.bin", "Graphic_66.bin", "AnimeInfo_4.bin", "Anime_4.bin"
    )]
    for directory in (root / "bin/pal", root / "map"):
        for path in sorted(directory.rglob("*")):
            if path.is_symlink():
                raise ValueError(f"Symlink not supported: {path}")
            if path.is_file():
                paths.append(path)
    result = []
    for path in sorted(paths):
        if path.is_symlink():
            raise ValueError(f"Symlink not supported: {path}")
        digest = hashlib.sha256()
        with path.open("rb") as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(block)
        result.append({"path": path.relative_to(root).as_posix(),
                       "size": path.stat().st_size, "sha256": digest.hexdigest()})
    return result


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: python3 scripts/verify_resources.py <Assets directory>")
    root = Path(sys.argv[1]).resolve(strict=True)
    repo = Path(__file__).resolve().parents[1]
    before = inventory(root)
    for entry in before:
        print(json.dumps(entry, ensure_ascii=False), flush=True)
    manifest = json.dumps(before, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    print("manifest_sha256=" + hashlib.sha256(manifest.encode()).hexdigest(), flush=True)
    completed = subprocess.run([
        "cargo", "run", "--locked", "--offline", "--release", "--example",
        "verify_resources", "--", str(root)
    ], cwd=repo, check=False)
    unchanged = before == inventory(root)
    print(f"inputs_unchanged={str(unchanged).lower()}", flush=True)
    print(f"audit_exit_code={completed.returncode}", flush=True)
    return completed.returncode if unchanged else 2


if __name__ == "__main__":
    sys.exit(main())
