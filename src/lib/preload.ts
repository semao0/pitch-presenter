// Keeps slide switching instant: every slide is fetched into the image cache
// up front, and the slides around the current one are kept decoded, so
// showing them never waits for the network or the PNG decoder.

import { slideUrl } from './api';

/** Slides kept decoded relative to the current one. */
const AROUND = [-1, 1, 2] as const;
/** Parallel fetches while warming the cache. */
const WARM_CONCURRENCY = 3;

export class SlidePreloader {
  #deckId = '';
  #decoded = new Map<string, HTMLImageElement>();
  #generation = 0;

  /** Starts warming the cache for a deck (no-op if it is already the current one). */
  setDeck(deckId: string, count: number): void {
    if (deckId === this.#deckId) return;
    this.#deckId = deckId;
    this.#decoded.clear();
    const generation = ++this.#generation;
    let next = 1;
    const pump = async (): Promise<void> => {
      while (next <= count && generation === this.#generation) {
        await load(slideUrl(deckId, next++));
      }
    };
    for (let i = 0; i < WARM_CONCURRENCY; i++) void pump();
  }

  /** Keeps the neighbors of `slide` decoded and releases the rest. */
  focus(slide: number, count: number): void {
    if (!this.#deckId) return;
    const wanted = new Set(
      AROUND.map((d) => slide + d)
        .filter((n) => n >= 1 && n <= count)
        .map((n) => slideUrl(this.#deckId, n)),
    );
    for (const url of this.#decoded.keys()) {
      if (!wanted.has(url)) this.#decoded.delete(url);
    }
    for (const url of wanted) {
      if (this.#decoded.has(url)) continue;
      const img = new Image();
      img.src = url;
      img.decode().catch(() => {});
      this.#decoded.set(url, img);
    }
  }
}

function load(url: string): Promise<void> {
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = img.onerror = () => resolve();
    img.src = url;
  });
}
