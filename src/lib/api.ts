// Typed access to the Rust backend: commands, events and slide URLs.

import { Channel, invoke } from '@tauri-apps/api/core';
import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Deck, LaunchInfo, Nav, Progress, ViewState } from './types';

export const MAIN_WINDOW = 'main';
export const AUDIENCE_WINDOW = 'audience';

export const api = {
  launchInfo: () => invoke<LaunchInfo>('launch_info'),

  /** Splits paths into `[deck, notes]` by extension. */
  classifyFiles: (paths: string[]) => invoke<[string | null, string | null]>('classify_files', { paths }),

  /** `talk.pdf` → `talk.toml`, if it exists. */
  guessNotes: (deck: string) => invoke<string | null>('guess_notes', { deck }),

  /** Renders the deck (streaming progress) and starts a session. */
  openDeck(deck: string, notes: string | null, minutes: number, onProgress: (p: Progress) => void) {
    const channel = new Channel<Progress>();
    channel.onmessage = onProgress;
    return invoke<Deck>('open_deck', { deck, notes, minutes, onProgress: channel });
  },

  closeDeck: () => invoke<void>('close_deck'),
  currentDeck: () => invoke<Deck | null>('current_deck'),
  currentView: () => invoke<ViewState | null>('current_view'),
  navigate: (nav: Nav) => invoke<ViewState>('navigate', { nav }),
  openAudience: () => invoke<void>('open_audience'),
  audienceOpen: () => invoke<boolean>('audience_open'),
};

/** Backend events. Each returns an unsubscribe function. */
export const events = {
  onView: (cb: (view: ViewState) => void) => subscribe<ViewState>('view', cb),
  onDeckUpdated: (cb: (deck: Deck) => void) => subscribe<Deck>('deck-updated', cb),
  onAudience: (cb: (open: boolean) => void) => subscribe<boolean>('audience', cb),
  onOpenFiles: (cb: (paths: string[]) => void) => subscribe<string[]>('open-files', cb),

  /** Keys pressed in the audience window, handled by the presenter. */
  onRemoteKey: (cb: (key: string) => void) => subscribe<string>('remote-key', cb),
  sendRemoteKey: (key: string) => emitTo(MAIN_WINDOW, 'remote-key', key),
};

function subscribe<T>(event: string, cb: (payload: T) => void): () => void {
  // `listen` is async; this wrapper lets components unsubscribe synchronously
  // from an effect cleanup, even before the listener is registered.
  let unlisten: UnlistenFn | undefined;
  let active = true;
  void listen<T>(event, (e) => cb(e.payload)).then((fn) => {
    if (active) unlisten = fn;
    else fn();
  });
  return () => {
    active = false;
    unlisten?.();
  };
}

// Custom protocols are `scheme://localhost` on macOS/Linux and
// `http://scheme.localhost` on Windows.
const SLIDE_ORIGIN = navigator.userAgent.includes('Windows') ? 'http://slide.localhost' : 'slide://localhost';

/** URL of a rendered slide. The deck id makes URLs unique per render. */
export function slideUrl(deckId: string, slide: number, thumb = false): string {
  return `${SLIDE_ORIGIN}/${deckId}/${thumb ? 'thumb/' : ''}${slide}`;
}

/** File name without its directory. */
export function baseName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** Directory part of a path. */
export function dirName(path: string): string {
  return path.replace(/[\\/][^\\/]*$/, '');
}
