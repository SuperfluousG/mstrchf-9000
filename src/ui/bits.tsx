import { catalogue } from '../data';
import { dietLabel } from '../domain/catalogue';
import type { Recipe } from '../domain/types';
import type { KeyboardEvent } from 'react';

export function choiceKeys(e: KeyboardEvent<HTMLElement>) {
  if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(e.key)) return;
  const buttons = [...e.currentTarget.querySelectorAll<HTMLButtonElement>('button[role="radio"], button[role="tab"]')].filter(b => !b.disabled);
  const index = buttons.indexOf(e.target as HTMLButtonElement);
  if (index < 0 || !buttons.length) return;
  e.preventDefault();
  const next = e.key === 'Home' ? 0 : e.key === 'End' ? buttons.length - 1
    : (index + (e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 1) + buttons.length) % buttons.length;
  buttons[next].focus(); buttons[next].click();
}

const HEAT_LABEL = ['No heat', 'Gentle warmth', 'Spicy', 'Hot'];

export function Heat({ level, showNone = false }: { level: number; showNone?: boolean }) {
  if (!level && !showNone) return null;
  return (
    <span className="heat" title={HEAT_LABEL[level]} aria-label={HEAT_LABEL[level]}>
      {[1, 2, 3].map(i => (
        <svg key={i} viewBox="0 0 12 16" className={i <= level ? 'on' : 'off'} aria-hidden="true">
          <path d="M7.5 1.2c.2 1-.3 1.6-1 1.9C9.6 3.5 11 6 10.6 9.2 10.1 13 7.4 15.2 4.6 14.8 2.4 14.5 1 12.8 1.2 10.9c.2-2.2 2-3.3 3-5 .5-.9.7-1.8.6-2.7.9 0 1.5-.6 1.6-1.6l1.1-.4z" />
        </svg>
      ))}
    </span>
  );
}

export function Meta({ recipe }: { recipe: Recipe }) {
  const diet = dietLabel(catalogue.facts.get(recipe.id)!);
  return (
    <span className="meta">
      <span>{recipe.time} min</span>
      {diet && <span className="diet">{diet}</span>}
      <Heat level={recipe.spice} />
    </span>
  );
}

export function Stepper({ value, min, max, onChange, label }: { value: number; min: number; max: number; onChange: (n: number) => void; label: string }) {
  return (
    <span className="stepper" role="group" aria-label={label}>
      <button type="button" onClick={() => onChange(Math.max(min, value - 1))} disabled={value <= min} aria-label={`Fewer ${label}`}>−</button>
      <output aria-live="polite">{value}</output>
      <button type="button" onClick={() => onChange(Math.min(max, value + 1))} disabled={value >= max} aria-label={`More ${label}`}>+</button>
    </span>
  );
}
