// Talk plan: how much time each slide gets, and how we are doing against it.

import type { Deck } from './types';

/** Speaking rate used when the talk length is not set, words per minute. */
export const WORDS_PER_MINUTE = 130;
/** Being within this of the plan counts as "on time", ms. */
const ON_TIME_MS = 5_000;
/** Falling behind by more than this is "late", ms. */
const LATE_MS = 30_000;

export interface Plan {
  /** Time budget per slide, ms; index = slide number (index 0 unused). */
  budget: number[];
  /** Planned elapsed time at the end of each slide, ms; `end[0] = 0`. */
  end: number[];
  /** Planned talk length, ms. */
  total: number;
}

/**
 * Splits the talk into per-slide budgets. Slides with an explicit `time` get
 * exactly that; the rest of the talk length is shared by the remaining slides
 * in proportion to their word count (equally if there are no words at all).
 * Without a talk length the total is derived from the words.
 */
export function computePlan(deck: Pick<Deck, 'count' | 'minutes' | 'notes'>): Plan {
  const { count } = deck;
  let fixedSum = 0;
  let wordSum = 0;
  let free = 0;
  const fixed: (number | null)[] = [null];
  const words: number[] = [0];
  for (let i = 1; i <= count; i++) {
    const note = deck.notes.slides[i];
    const seconds = note?.seconds ?? null;
    fixed[i] = seconds === null ? null : seconds * 1000;
    words[i] = note?.words ?? 0;
    if (seconds === null) {
      wordSum += words[i]!;
      free++;
    } else {
      fixedSum += seconds * 1000;
    }
  }

  const total = deck.minutes > 0 ? deck.minutes * 60_000 : fixedSum + (wordSum / WORDS_PER_MINUTE) * 60_000;
  const pool = Math.max(0, total - fixedSum);
  const budget = [0];
  const end = [0];
  for (let i = 1; i <= count; i++) {
    const own = fixed[i];
    budget[i] = own ?? (wordSum > 0 ? (pool * words[i]!) / wordSum : free > 0 ? pool / free : 0);
    end[i] = end[i - 1]! + budget[i]!;
  }
  return { budget, end, total };
}

export type PaceStatus = 'onTime' | 'ahead' | 'behind' | 'late';

export interface Pace {
  status: PaceStatus;
  /** Positive when behind the plan, ms. */
  diff: number;
}

/**
 * Where we are relative to the plan: the planned time at this point is the
 * end of the previous slide plus the time spent here, capped by its budget.
 */
export function pace(plan: Plan, slide: number, elapsed: number, spentHere: number): Pace | null {
  const budget = plan.budget[slide];
  if (!budget) return null;
  const planned = plan.end[slide - 1]! + Math.min(spentHere, budget);
  const diff = elapsed - planned;
  const status: PaceStatus =
    Math.abs(diff) < ON_TIME_MS ? 'onTime' : diff < 0 ? 'ahead' : diff > LATE_MS ? 'late' : 'behind';
  return { status, diff };
}

/** `mm:ss` of the absolute value; hours roll into minutes. */
export function formatDuration(ms: number): string {
  const total = Math.floor(Math.abs(ms) / 1000);
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
}
