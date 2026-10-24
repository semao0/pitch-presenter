import { describe, expect, it } from 'vitest';
import { finished, followPosition, similarity, words } from './speech';

const script = words(
  'Происходит ли сейчас на площадке то, что должно быть по плану? Большинство руководителей ' +
    'узнают об этом слишком поздно, когда сроки уже сорваны, а бюджет перерасходован.',
);

describe('similarity', () => {
  it('forgives endings but not short words', () => {
    expect(similarity('руководителей', 'руководители')).toBeGreaterThanOrEqual(0.75);
    expect(similarity('сорваны', 'сорван')).toBeGreaterThanOrEqual(0.75);
    expect(similarity('на', 'но')).toBe(0);
    expect(similarity('бюджет', 'площадке')).toBeLessThan(0.5);
  });
});

describe('followPosition', () => {
  it('follows exact speech', () => {
    expect(followPosition(script, 'Происходит ли сейчас на площадке', 0)).toBe(5);
  });

  it('follows speech with wrong endings', () => {
    const heard = 'большинство руководители узнаю об этом слишком поздно';
    expect(followPosition(script, heard, 12)).toBe(18);
  });

  it('never moves back', () => {
    expect(followPosition(script, 'происходит ли сейчас на площадке', 20)).toBe(20);
  });

  it('ignores unrelated speech', () => {
    expect(followPosition(script, 'давайте перейдём к следующему вопросу', 5)).toBe(5);
  });
});

describe('finished', () => {
  it('needs the whole short text, the last word of a long one', () => {
    expect(finished(4, 5)).toBe(false);
    expect(finished(5, 5)).toBe(true);
    expect(finished(19, 20)).toBe(true);
    expect(finished(18, 20)).toBe(false);
  });

  it('never fires on empty or two-word slides', () => {
    expect(finished(0, 0)).toBe(false);
    expect(finished(2, 2)).toBe(false);
  });
});
