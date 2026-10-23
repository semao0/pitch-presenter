// Keyboard shortcuts that work in any layout.
//
// `KeyboardEvent.key` follows the active layout: with a Russian layout the "G"
// key reports "п", so letter shortcuts would silently stop working. Letters
// and punctuation are therefore taken from the physical key (`code`), while
// named keys (arrows, Enter, digits) come from `key` as usual.

const PUNCTUATION: Record<string, [plain: string, shifted: string]> = {
  Period: ['.', '>'],
  Slash: ['/', '?'],
  Equal: ['=', '+'],
  Minus: ['-', '_'],
  NumpadAdd: ['+', '+'],
  NumpadSubtract: ['-', '-'],
};

/** The layout-independent name of a pressed key: `g`, `R` (with Shift), `?`, `ArrowRight`, `5`… */
export function shortcutKey(e: Pick<KeyboardEvent, 'key' | 'code' | 'shiftKey'>): string {
  const letter = /^Key([A-Z])$/.exec(e.code)?.[1];
  if (letter) return e.shiftKey ? letter : letter.toLowerCase();
  const punctuation = PUNCTUATION[e.code];
  if (punctuation) return punctuation[e.shiftKey ? 1 : 0];
  return e.key;
}
