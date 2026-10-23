import { describe, expect, it } from 'vitest';
import { shortcutKey } from './keys';

const key = (e: { key: string; code: string; shiftKey?: boolean }) => shortcutKey({ shiftKey: false, ...e });

describe('shortcutKey', () => {
  it('reads letters from the physical key, whatever the layout', () => {
    expect(key({ key: 'п', code: 'KeyG' })).toBe('g');
    expect(key({ key: 'К', code: 'KeyR', shiftKey: true })).toBe('R');
  });

  it('maps punctuation by position', () => {
    expect(key({ key: 'ю', code: 'Period' })).toBe('.');
    expect(key({ key: ',', code: 'Slash', shiftKey: true })).toBe('?');
    expect(key({ key: '+', code: 'Equal', shiftKey: true })).toBe('+');
  });

  it('passes named keys and digits through', () => {
    expect(key({ key: 'ArrowRight', code: 'ArrowRight' })).toBe('ArrowRight');
    expect(key({ key: '7', code: 'Digit7' })).toBe('7');
    expect(key({ key: ' ', code: 'Space' })).toBe(' ');
  });
});
