"""Imports recipe photos into src/photos/<recipe id>.webp.

Usage: python scripts/import_photos.py <folder> [--force]

Every PNG, JPEG or WebP in <folder> named after a recipe id is centre-cropped to 3:2,
resized to 960x640 and saved as WebP. Files that match no recipe are listed and skipped.
Existing photos are kept unless --force is given.
"""
import json
import pathlib
import sys

from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "src" / "photos"
SIZE = (960, 640)


def recipe_ids() -> set[str]:
    ids: set[str] = set()
    for f in (ROOT / "catalogue" / "recipes").glob("*.json"):
        ids.update(r["id"] for r in json.loads(f.read_text(encoding="utf-8")))
    return ids


def to_card(img: Image.Image) -> Image.Image:
    img = img.convert("RGB")
    w, h = img.size
    target = SIZE[0] / SIZE[1]
    if w / h > target:
        nw = round(h * target)
        img = img.crop(((w - nw) // 2, 0, (w - nw) // 2 + nw, h))
    else:
        nh = round(w / target)
        img = img.crop((0, (h - nh) // 2, w, (h - nh) // 2 + nh))
    return img.resize(SIZE, Image.LANCZOS)


def main() -> None:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    force = "--force" in sys.argv
    if len(args) != 1:
        sys.exit(__doc__)
    src = pathlib.Path(args[0])
    ids = recipe_ids()
    OUT.mkdir(parents=True, exist_ok=True)
    done, kept, unknown = [], [], []
    for f in sorted(src.iterdir()):
        if f.suffix.lower() not in {".png", ".jpg", ".jpeg", ".webp"}:
            continue
        if f.stem not in ids:
            unknown.append(f.name)
            continue
        dest = OUT / f"{f.stem}.webp"
        if dest.exists() and not force:
            kept.append(f.stem)
            continue
        with Image.open(f) as img:
            to_card(img).save(dest, "WEBP", quality=80, method=6)
        done.append(f.stem)
    total = len(list(OUT.glob("*.webp")))
    print(f"Imported {len(done)}, kept {len(kept)} existing, {len(unknown)} not recipes. {total} of {len(ids)} recipes now have photos.")
    if unknown:
        print("Not recipe ids:", ", ".join(unknown))


if __name__ == "__main__":
    main()
