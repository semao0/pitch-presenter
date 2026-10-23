// Mirrors of the Rust types sent over IPC (see src-tauri/src). Keep in sync.

/** A span of note text (`notes/markup.rs: Inline`). */
export type Inline =
  | { kind: 'text'; text: string }
  | { kind: 'code'; text: string }
  | { kind: 'strong' | 'em' | 'mark'; children: Inline[] };

/** A paragraph-level element of a note (`notes/markup.rs: Block`). */
export type Block =
  | { kind: 'paragraph'; content: Inline[] }
  | { kind: 'list'; content: Inline[][] }
  | { kind: 'cue'; content: Inline[] };

export interface SlideNote {
  title: string;
  blocks: Block[];
  /** Spoken words, used to split the talk time between slides. */
  words: number;
  /** Explicit time limit, seconds. */
  seconds: number | null;
  reserve: boolean;
}

export interface Notes {
  title: string | null;
  minutes: number | null;
  /** Keyed by 1-based slide number. */
  slides: Partial<Record<number, SlideNote>>;
  warnings: string[];
}

export interface Deck {
  id: string;
  title: string;
  count: number;
  /** Width / height of the slides. */
  aspect: number;
  /** Talk length; 0 means "no limit". */
  minutes: number;
  notes: Notes;
  notesFile: string | null;
  /** The notes file is broken; the last good notes are still shown. */
  notesError: string | null;
}

/** What the audience sees. */
export interface ViewState {
  deckId: string;
  count: number;
  slide: number;
  blank: boolean;
}

export type Nav =
  | { type: 'next' }
  | { type: 'prev' }
  | { type: 'first' }
  | { type: 'last' }
  | { type: 'goto'; slide: number }
  | { type: 'setBlank'; blank: boolean };

export type Progress = { stage: 'converting' } | { stage: 'rendering'; done: number; total: number };

export interface RecentDeck {
  deck: string;
  notes: string | null;
  /** Unix time, seconds. */
  openedAt: number;
}

export interface LaunchInfo {
  deck: string | null;
  notes: string | null;
  minutes: number;
  recent: RecentDeck[];
}
