import { describe, expect, it } from 'vitest';
import { computePlan, formatDuration, pace, WORDS_PER_MINUTE } from './plan';
import type { Notes, SlideNote } from './types';

function note(words: number, seconds: number | null = null): SlideNote {
  return { title: '', blocks: [], words, seconds, reserve: false };
}

function notes(slides: Notes['slides']): Notes {
  return { title: null, minutes: null, slides, warnings: [] };
}

describe('computePlan', () => {
  it('splits the talk length by words, honoring fixed times', () => {
    const plan = computePlan({ count: 3, minutes: 2, notes: notes({ 1: note(10), 2: note(30), 3: note(99, 20) }) });
    expect(plan.budget.slice(1)).toEqual([25_000, 75_000, 20_000]);
    expect(plan.end).toEqual([0, 25_000, 100_000, 120_000]);
    expect(plan.total).toBe(120_000);
  });

  it('derives the length from words when minutes are not set', () => {
    const plan = computePlan({ count: 2, minutes: 0, notes: notes({ 1: note(WORDS_PER_MINUTE), 2: note(0, 15) }) });
    expect(plan.budget.slice(1)).toEqual([60_000, 15_000]);
  });

  it('shares time equally when there are no words', () => {
    const plan = computePlan({ count: 4, minutes: 4, notes: notes({}) });
    expect(plan.budget.slice(1)).toEqual([60_000, 60_000, 60_000, 60_000]);
  });

  it('never gives negative budgets when fixed times exceed the length', () => {
    const plan = computePlan({ count: 2, minutes: 1, notes: notes({ 1: note(0, 90), 2: note(10) }) });
    expect(plan.budget.slice(1)).toEqual([90_000, 0]);
  });
});

describe('pace', () => {
  const plan = computePlan({ count: 2, minutes: 2, notes: notes({}) });

  it('classifies the difference from the plan', () => {
    expect(pace(plan, 2, 62_000, 0)?.status).toBe('onTime');
    expect(pace(plan, 2, 40_000, 0)?.status).toBe('ahead');
    expect(pace(plan, 2, 75_000, 0)?.status).toBe('behind');
    expect(pace(plan, 1, 45_000, 45_000)?.status).toBe('onTime');
    expect(pace(plan, 1, 100_000, 100_000)?.status).toBe('late');
  });

  it('is null without a budget', () => {
    expect(pace(computePlan({ count: 1, minutes: 0, notes: notes({}) }), 1, 0, 0)).toBeNull();
  });
});

describe('formatDuration', () => {
  it('formats mm:ss', () => {
    expect(formatDuration(0)).toBe('00:00');
    expect(formatDuration(65_999)).toBe('01:05');
    expect(formatDuration(-61_000)).toBe('01:01');
    expect(formatDuration(3_600_000)).toBe('60:00');
  });
});
