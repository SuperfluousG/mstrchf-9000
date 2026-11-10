import { describe, expect, it, vi } from 'vitest';
import { DEFAULT_PROFILE } from '../src/domain/profile';
import * as fileSync from '../src/state/fileSync';
import { EMPTY_STATE, getState, importBackup, newWeek, readBackup, setState } from '../src/state/store';

describe('backup recovery', () => {
  it('rejects incomplete version-2 files without replacing the current kitchen', () => {
    setState(() => ({ ...structuredClone(EMPTY_STATE), profile: { ...DEFAULT_PROFILE, people: 3 } }));
    const before = getState();
    for (const state of [{ version: 2 }, { version: 2, profile: null, weeks: [] }, { ...EMPTY_STATE, profile: { people: 2 } }, { ...EMPTY_STATE, favourites: null }]) {
      expect(() => importBackup(JSON.stringify({ state }))).toThrow(/backup/);
      expect(getState()).toBe(before);
    }
  });
  it('reads existing version-2 backups, including older ordered weeks with no saved basket', () => {
    const state = { ...structuredClone(EMPTY_STATE), profile: structuredClone(DEFAULT_PROFILE), weeks: { '2026-11-16': { ...newWeek('2026-11-16'), status: 'ordered' } } };
    expect(readBackup(JSON.stringify({ app: 'MSTRCHF 9000', state }))).toEqual(state);
  });
  it('rejects malformed meal picks before any state change', () => {
    const state = { ...structuredClone(EMPTY_STATE), profile: structuredClone(DEFAULT_PROFILE), weeks: { '2026-11-16': { ...newWeek('2026-11-16'), picks: [{ uid: 'x', recipeId: 'unknown', meal: 'dinner', day: 12, reasons: [] }] } } };
    expect(() => readBackup(JSON.stringify({ state }))).toThrow(/backup/);
  });
  it('rejects malformed optional pantry and taste data before it can break a restored kitchen', () => {
    const state = { ...structuredClone(EMPTY_STATE), profile: structuredClone(DEFAULT_PROFILE), weeks: { '2026-11-16': newWeek('2026-11-16') } };
    expect(() => readBackup(JSON.stringify({ ...state, weeks: { '2026-11-16': { ...state.weeks['2026-11-16'], pantryToBuy: 42 } } }))).toThrow(/backup/);
    expect(() => readBackup(JSON.stringify({ ...state, profile: { ...state.profile, cuisines: { thai: 'love' } } }))).toThrow(/backup/);
  });
  it('restores an empty backup with the explicit reset authority required by the external store', () => {
    const save = vi.spyOn(fileSync, 'scheduleSave');
    try {
      importBackup(JSON.stringify({ app: 'MSTRCHF 9000', state: EMPTY_STATE }));
      expect(getState()).toMatchObject({ profile: null, weeks: {} });
      expect(save).toHaveBeenCalledWith(getState(), getState().savedAt, true);
    } finally { save.mockRestore(); }
  });
});
