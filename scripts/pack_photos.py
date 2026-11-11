"""Packs src/photos/*.webp into src/photo-packs/ for the Claude artifact build.

Each recipe photo is resized to 720 px wide (WebP, quality 72) and base64-encoded into a few JSON files
({"<recipe id>": "<base64 WebP>"}) of about 7.5 MB, plus index.json ({"packs": n, "ids": {"<recipe id>": n}}).
A Claude install then publishes about 15 files instead of about 400. src/photoSource.packed.ts loads them.
Skips the work when the packs are newer than every photo and this script.
"""
import base64
import io
import json
import pathlib

from PIL import Image

WIDTH, QUALITY, PACK_CHARS = 720, 72, 7_500_000

root = pathlib.Path(__file__).resolve().parent.parent
src = root / "src" / "photos"
out = root / "src" / "photo-packs"
photos = sorted(src.glob("*.webp"))
index_file = out / "index.json"

newest_input = max(p.stat().st_mtime for p in [*photos, pathlib.Path(__file__)])
if index_file.exists() and index_file.stat().st_mtime > newest_input:
    print(f"Photo packs are up to date in {out.relative_to(root)}.")
    raise SystemExit(0)

out.mkdir(exist_ok=True)
for old in out.glob("*.json"):
    old.unlink()

packs: list[dict[str, str]] = [{}]
ids: dict[str, int] = {}
chars = 0
for path in photos:
    image = Image.open(path).convert("RGB")
    height = round(WIDTH * image.height / image.width)
    buffer = io.BytesIO()
    image.resize((WIDTH, height), Image.LANCZOS).save(buffer, "WEBP", quality=QUALITY, method=6)
    encoded = base64.b64encode(buffer.getvalue()).decode("ascii")
    if packs[-1] and chars + len(encoded) > PACK_CHARS:
        packs.append({})
        chars = 0
    packs[-1][path.stem] = encoded
    ids[path.stem] = len(packs) - 1
    chars += len(encoded) + len(path.stem) + 6

for n, pack in enumerate(packs):
    (out / f"photos-{n}.json").write_text(json.dumps(pack, separators=(",", ":")), encoding="ascii")
index_file.write_text(json.dumps({"packs": len(packs), "ids": ids}, separators=(",", ":")), encoding="ascii")

total = sum((out / f"photos-{n}.json").stat().st_size for n in range(len(packs)))
print(f"Packed {len(ids)} photos into {len(packs)} files in {out.relative_to(root)} ({total / 1024 / 1024:.1f} MB).")
