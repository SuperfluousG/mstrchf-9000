"""Packs plugin/ into release/mstrchf-9000-<version>.plugin (a zip with the plugin files at its root)."""
import json
import pathlib
import zipfile

root = pathlib.Path(__file__).resolve().parent.parent
src = root / "plugin"
version = json.loads((src / ".claude-plugin" / "plugin.json").read_text(encoding="utf-8"))["version"]
out = root / "release" / f"mstrchf-9000-{version}.plugin"
out.parent.mkdir(exist_ok=True)

with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    for path in sorted(src.rglob("*")):
        if path.is_file():
            z.write(path, path.relative_to(src).as_posix())

print(f"Packed {out.relative_to(root)} ({out.stat().st_size / 1024 / 1024:.1f} MB)")
