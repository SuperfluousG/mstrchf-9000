import { useEffect, useMemo, useRef, useState } from 'react';
import { codexPhoto, isCodexApp } from '../codexHost';
import { catalogue, imageUrl } from '../data';
import type { Recipe } from '../domain/types';

const SKIP_AISLES = new Set(['spices', 'sauces', 'cupboard']);
const SPECKLE_AISLES = new Set(['herbs']);
const SPECKLE_IDS = new Set(['spring-onion', 'red-chilli', 'green-chilli', 'chilli-flakes', 'sesame-seeds', 'pomegranate', 'parmesan', 'feta']);

function rng(seed: string) {
  let h = 1779033703 ^ seed.length;
  for (let i = 0; i < seed.length; i++) { h = Math.imul(h ^ seed.charCodeAt(i), 3432918353); h = (h << 13) | (h >>> 19); }
  return () => {
    h = Math.imul(h ^ (h >>> 16), 2246822507); h = Math.imul(h ^ (h >>> 13), 3266489909);
    return ((h ^= h >>> 16) >>> 0) / 4294967296;
  };
}

function blob(cx: number, cy: number, r: number, rand: () => number): string {
  const n = 9;
  const pts = Array.from({ length: n }, (_, i) => {
    const a = (i / n) * Math.PI * 2;
    const rr = r * (0.82 + rand() * 0.36);
    return [cx + Math.cos(a) * rr, cy + Math.sin(a) * rr * 0.92];
  });
  let d = '';
  for (let i = 0; i < n; i++) {
    const p = pts[i], q = pts[(i + 1) % n];
    const mid = [(p[0] + q[0]) / 2, (p[1] + q[1]) / 2];
    d += i === 0 ? `M${((pts[n - 1][0] + p[0]) / 2).toFixed(1)},${((pts[n - 1][1] + p[1]) / 2).toFixed(1)}` : '';
    d += ` Q${p[0].toFixed(1)},${p[1].toFixed(1)} ${mid[0].toFixed(1)},${mid[1].toFixed(1)}`;
  }
  return d + 'Z';
}

const AROMATICS = new Set(['onion', 'red-onion', 'shallot', 'garlic', 'ginger', 'lemon', 'lime', 'lemongrass', 'celery', 'veg-stock', 'chicken-stock', 'beef-stock']);
const SAUCES = new Set(['chopped-tomatoes', 'passata', 'tomato-puree', 'coconut-milk', 'double-cream', 'creme-fraiche', 'milk', 'oat-milk', 'natural-yoghurt']);
const BASES = new Set(['grains', 'bakery']);
const BASE_IDS = new Set(['potato', 'baby-potato', 'sweet-potato', 'gnocchi', 'cauliflower']);
const PROTEIN_KINDS = new Set(['meat', 'poultry', 'fish', 'shellfish']);
const PROTEIN_IDS = new Set(['tofu', 'paneer', 'halloumi', 'egg', 'chickpeas', 'black-beans', 'kidney-beans', 'cannellini-beans', 'butter-beans', 'feta', 'mozzarella', 'red-lentils', 'puy-lentils']);

type Piece = { d: string; fill: string; opacity?: number };

/** A generated top-down plate: sauce wash, a bed of carbs, scattered protein and veg, herbs on top. */
function Plate({ recipe }: { recipe: Recipe }) {
  const art = useMemo(() => {
    const rand = rng(recipe.id);
    const parts = recipe.ingredients
      .map(ri => ({ ri, ing: catalogue.ingredients.get(ri.id)! }))
      .filter(x => x.ing && x.ing.colour && !x.ing.pantry && !SKIP_AISLES.has(x.ing.aisle) && !AROMATICS.has(x.ing.id));
    const weight = (x: (typeof parts)[number]) => (x.ing.unit === 'each' ? x.ri.qty * 90 : x.ri.qty);
    const isSpeckle = (x: (typeof parts)[number]) => SPECKLE_AISLES.has(x.ing.aisle) || SPECKLE_IDS.has(x.ing.id);
    const sauce = parts.filter(x => SAUCES.has(x.ing.id)).sort((a, b) => weight(b) - weight(a))[0];
    const base = parts.filter(x => BASES.has(x.ing.aisle) || BASE_IDS.has(x.ing.id)).sort((a, b) => weight(b) - weight(a))[0];
    const proteins = parts.filter(x => !isSpeckle(x) && (PROTEIN_KINDS.has(x.ing.kind) || PROTEIN_IDS.has(x.ing.id)) && x !== base).slice(0, 2);
    const veg = parts.filter(x => !isSpeckle(x) && x !== base && x !== sauce && !proteins.includes(x) && !SAUCES.has(x.ing.id))
      .sort((a, b) => weight(b) - weight(a)).slice(0, 3);
    const speckles = parts.filter(isSpeckle);

    const pieces: Piece[] = [];
    if (sauce) pieces.push({ d: blob(80, 60, 33, rand), fill: sauce.ing.colour!, opacity: 0.55 });
    // Bed sits to one side when there is a sauce, centred otherwise.
    const bx = sauce ? 66 + rand() * 6 : 80, by = sauce ? 56 + rand() * 8 : 60;
    if (base) pieces.push({ d: blob(bx, by, sauce ? 20 : 26, rand), fill: base.ing.colour! });
    const scatter = (fill: string, n: number, size: number, spread: number, cx = 80, cy = 60) => {
      for (let i = 0; i < n; i++) {
        const a = rand() * Math.PI * 2, dist = Math.sqrt(rand()) * spread;
        pieces.push({ d: blob(cx + Math.cos(a) * dist, cy + Math.sin(a) * dist * 0.9, size * (0.7 + rand() * 0.6), rand), fill });
      }
    };
    const px = base && sauce ? 92 : 80;
    proteins.forEach((p, i) => scatter(p.ing.colour!, i === 0 ? 6 : 4, i === 0 ? 7 : 5.5, base ? 20 : 26, px, 60));
    veg.forEach(v => scatter(v.ing.colour!, 5, 4.2, 30));
    if (!base && !proteins.length && !veg.length) scatter('#C9B98F', 7, 8, 24);
    const dots = speckles.flatMap(x => Array.from({ length: 10 }, () => {
      const a = rand() * Math.PI * 2, d = Math.sqrt(rand()) * 31;
      return { cx: 80 + Math.cos(a) * d, cy: 60 + Math.sin(a) * d * 0.92, rx: 1 + rand() * 1.5, ry: 0.6 + rand() * 0.8, rot: rand() * 180, fill: x.ing.colour! };
    }));
    return { pieces, dots, tint: (proteins[0] ?? base ?? veg[0])?.ing.colour ?? '#c9b98f' };
  }, [recipe]);

  return (
    <svg viewBox="0 0 160 120" className="dish-art" role="img" aria-label={`Illustration of ${recipe.title}`} preserveAspectRatio="xMidYMid slice">
      <rect width="160" height="120" fill="var(--chalk-deep)" />
      <rect width="160" height="120" fill={art.tint} opacity="0.12" />
      <ellipse cx="83" cy="65" rx="47" ry="45" fill="#000" opacity="0.07" />
      <circle cx="80" cy="60" r="46" fill="#FBFAF5" />
      <circle cx="80" cy="60" r="39" fill="none" stroke="#E7E3D6" strokeWidth="1.2" />
      {art.pieces.map((b, i) => <path key={i} d={b.d} fill={b.fill} opacity={b.opacity ?? 0.95} stroke="rgba(0,0,0,.06)" strokeWidth="0.5" />)}
      {art.dots.map((d, i) => <ellipse key={i} cx={d.cx} cy={d.cy} rx={d.rx} ry={d.ry} fill={d.fill} transform={`rotate(${d.rot} ${d.cx} ${d.cy})`} />)}
    </svg>
  );
}

export function DishArt({ recipe }: { recipe: Recipe }) {
  if (isCodexApp) return <CodexDish recipe={recipe} />;
  const src = imageUrl(recipe);
  if (src) return <img className="dish-photo" src={src} alt={recipe.title} loading="lazy" />;
  return <Plate recipe={recipe} />;
}

function CodexDish({ recipe }: { recipe: Recipe }) {
  const ref = useRef<HTMLDivElement>(null);
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    const observer = new IntersectionObserver(entries => {
      if (!entries.some(e => e.isIntersecting)) return;
      observer.disconnect();
      void codexPhoto(recipe.id).then(url => { if (live) setSrc(url); }).catch(() => {});
    }, { rootMargin: '200px' });
    if (ref.current) observer.observe(ref.current);
    return () => { live = false; observer.disconnect(); };
  }, [recipe.id]);
  return <div className="codex-photo" ref={ref}>{src ? <img className="dish-photo" src={src} alt={recipe.title} /> : <Plate recipe={recipe} />}</div>;
}
