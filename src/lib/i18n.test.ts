import { describe, expect, it } from 'vitest';
import { dictionaries, format } from './i18n';

describe('i18n', () => {
  it('has the same keys in every language', () => {
    expect(Object.keys(dictionaries.en).sort()).toEqual(Object.keys(dictionaries.ru).sort());
  });

  it('uses the same placeholders in every language', () => {
    const placeholders = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort();
    for (const [key, ru] of Object.entries(dictionaries.ru)) {
      expect(placeholders(dictionaries.en[key as keyof typeof dictionaries.en]), key).toEqual(placeholders(ru));
    }
  });

  it('fills placeholders and leaves unknown ones', () => {
    expect(format('Слайд {n} из {total}', { n: 3 })).toBe('Слайд 3 из {total}');
  });
});
