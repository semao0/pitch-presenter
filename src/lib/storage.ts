// Small JSON wrappers over Web Storage that never throw.

function store(area: () => Storage) {
  return {
    get<T>(key: string, fallback: T): T {
      try {
        const raw = area().getItem(`pp.${key}`);
        return raw === null ? fallback : (JSON.parse(raw) as T);
      } catch {
        return fallback;
      }
    },
    set(key: string, value: unknown): void {
      try {
        area().setItem(`pp.${key}`, JSON.stringify(value));
      } catch {
        // Storage full or unavailable: preferences are a convenience only.
      }
    },
  };
}

/** Kept across launches: font size, layout. */
export const prefs = store(() => localStorage);
/** Kept until the app quits: the talk timer survives a page reload. */
export const session = store(() => sessionStorage);
