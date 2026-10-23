// The talk timer: total elapsed time plus time spent on each slide.

import { session } from './storage';

interface Saved {
  acc: number;
  startedAt: number | null;
  spent: Record<number, number>;
}

export class TalkTimer {
  /** Accumulated time before the current run, ms. */
  #acc = $state(0);
  /** Epoch ms when the current run started; `null` while paused. */
  #startedAt = $state<number | null>(null);
  /** Time spent on each slide, ms. */
  #spent = $state<Record<number, number>>({});
  /** Wall clock, refreshed by `tick`; everything time-based derives from it. */
  now = $state(Date.now());

  #key: string;
  #lastTick = Date.now();

  constructor(deckId: string) {
    this.#key = `timer.${deckId}`;
    const saved = session.get<Saved | null>(this.#key, null);
    if (saved) {
      this.#acc = saved.acc;
      this.#startedAt = saved.startedAt;
      this.#spent = saved.spent;
    }
  }

  get running(): boolean {
    return this.#startedAt !== null;
  }

  /** Never started or reset. */
  get pristine(): boolean {
    return !this.running && this.#acc === 0;
  }

  get elapsed(): number {
    return this.#acc + (this.#startedAt === null ? 0 : this.now - this.#startedAt);
  }

  spentOn(slide: number): number {
    return this.#spent[slide] ?? 0;
  }

  toggle(): void {
    const now = Date.now();
    this.now = now;
    this.#lastTick = now;
    if (this.#startedAt === null) {
      this.#startedAt = now;
    } else {
      this.#acc += now - this.#startedAt;
      this.#startedAt = null;
    }
    this.#save();
  }

  reset(): void {
    this.#acc = 0;
    this.#startedAt = null;
    this.#spent = {};
    this.#save();
  }

  /** Advances the clock and credits the time since the last tick to `slide`. */
  tick(slide: number): void {
    const now = Date.now();
    if (this.running) {
      this.#spent[slide] = (this.#spent[slide] ?? 0) + (now - this.#lastTick);
      this.#save();
    }
    this.#lastTick = now;
    this.now = now;
  }

  #save(): void {
    session.set(this.#key, { acc: this.#acc, startedAt: this.#startedAt, spent: this.#spent } satisfies Saved);
  }
}
