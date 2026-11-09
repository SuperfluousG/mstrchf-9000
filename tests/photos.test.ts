import { readdirSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const recipeIds = readdirSync('catalogue/recipes')
  .flatMap(f => JSON.parse(readFileSync(`catalogue/recipes/${f}`, 'utf8')) as { id: string }[])
  .map(r => r.id);
const photoIds = readdirSync('src/photos').filter(f => f.endsWith('.webp')).map(f => f.slice(0, -'.webp'.length));

describe('recipe photos', () => {
  it('has a photo for every recipe', () => {
    expect(recipeIds.filter(id => !photoIds.includes(id))).toEqual([]);
  });

  it('has no photos for recipes that do not exist', () => {
    expect(photoIds.filter(id => !recipeIds.includes(id))).toEqual([]);
  });
});
