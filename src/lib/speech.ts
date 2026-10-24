import type { Block, Inline } from './types';

export function words(text: string): string[] {
  return text.toLowerCase().replaceAll('ё', 'е').match(/[\p{L}\p{N}]+/gu) ?? [];
}

export function inlineWords(nodes: Inline[]): string[] {
  return nodes.flatMap((node) => 'text' in node ? words(node.text) : inlineWords(node.children));
}

export function noteWords(blocks: Block[]): string[] {
  return blocks.flatMap((block) => block.kind === 'cue' ? [] :
    block.kind === 'list' ? block.content.flatMap(inlineWords) : inlineWords(block.content));
}

export function inlineOffsets(nodes: Inline[], start = 0): number[] {
  let offset = start;
  return nodes.map((node) => {
    const previous = offset;
    offset += inlineWords([node]).length;
    return previous;
  });
}

export function blockOffsets(blocks: Block[]): number[] {
  let offset = 0;
  return blocks.map((block) => {
    const previous = offset;
    offset += noteWords([block]).length;
    return previous;
  });
}

function distance(a: string, b: string): number {
  let previous = Array.from({ length: b.length + 1 }, (_, i) => i);
  for (let i = 1; i <= a.length; i++) {
    const current = [i];
    for (let j = 1; j <= b.length; j++) {
      current[j] = Math.min(previous[j]! + 1, current[j - 1]! + 1,
        previous[j - 1]! + (a[i - 1] === b[j - 1] ? 0 : 1));
    }
    previous = current;
  }
  return previous[b.length]!;
}

/**
 * How alike two words are, 1 — the same word. The model often gets endings
 * wrong («руководителей» → «руководители»), so a shared stem of four letters
 * counts as a match; words under three letters must match exactly.
 */
export function similarity(a: string, b: string): number {
  if (a === b) return 1;
  if (Math.min(a.length, b.length) < 3) return 0;
  let prefix = 0;
  while (prefix < a.length && a[prefix] === b[prefix]) prefix++;
  return Math.max(1 - distance(a, b) / Math.max(a.length, b.length), prefix >= 4 ? 0.85 : 0);
}

export function followPosition(script: string[], transcript: string, position: number): number {
  const spoken = words(transcript).slice(-40);
  if (spoken.length < 3) return position;
  const start = Math.max(0, position - 16);
  const target = script.slice(start, position + 100);
  let previous = new Float64Array(target.length + 1);
  let previousMatches = new Uint16Array(target.length + 1);
  let scores = new Float64Array(target.length + 1);
  let matches = new Uint16Array(target.length + 1);
  let bestScore = 0;
  let bestPosition = position;
  for (const word of spoken) {
    for (let column = 1; column <= target.length; column++) {
      const alike = similarity(word, target[column - 1]!);
      const equal = alike >= 0.75;
      const diagonal = previous[column - 1]! + (equal ? 3 : alike >= 0.5 ? 0 : -2);
      const above = previous[column]! - 2;
      const left = scores[column - 1]! - 2;
      scores[column] = Math.max(0, diagonal, above, left);
      matches[column] = scores[column] === 0 ? 0 : scores[column] === diagonal
        ? previousMatches[column - 1]! + Number(equal)
        : scores[column] === above ? previousMatches[column]! : matches[column - 1]!;
      const candidate = start + column;
      if (equal && matches[column]! >= 3 && scores[column]! >= 8 &&
          scores[column]! > bestScore) {
        bestScore = scores[column]!;
        bestPosition = candidate;
      }
    }
    [previous, scores] = [scores, previous];
    [previousMatches, matches] = [matches, previousMatches];
  }
  return Math.max(position, bestPosition);
}

/**
 * Whether the speaker has read the slide's text through. The last word is
 * often still being recognized, so in a longer text standing on it is enough;
 * a couple of words («Спасибо!») never count.
 */
export function finished(position: number, total: number): boolean {
  if (total < 3) return false;
  return position >= total - (total > 8 ? 1 : 0);
}
