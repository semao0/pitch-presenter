// Appearance and language, shared by every component and persisted.

import { dictionaries, format, systemLang, type Lang, type MessageKey } from './i18n';
import { prefs } from './storage';

export type Theme = 'light' | 'dark';

const darkQuery = matchMedia('(prefers-color-scheme: dark)');

class Settings {
  /** Explicit choice; `null` follows the system. */
  #theme = $state<Theme | null>(prefs.get<Theme | null>('theme', null));
  #systemDark = $state(darkQuery.matches);
  #lang = $state<Lang>(prefs.get<Lang | null>('lang', null) ?? systemLang());

  constructor() {
    darkQuery.addEventListener('change', (e) => (this.#systemDark = e.matches));
  }

  get theme(): Theme {
    return this.#theme ?? (this.#systemDark ? 'dark' : 'light');
  }

  toggleTheme(): void {
    this.#theme = this.theme === 'dark' ? 'light' : 'dark';
    prefs.set('theme', this.#theme);
  }

  get lang(): Lang {
    return this.#lang;
  }

  toggleLang(): void {
    this.#lang = this.#lang === 'ru' ? 'en' : 'ru';
    prefs.set('lang', this.#lang);
  }
}

export const settings = new Settings();

/** Translates a UI string in the current language (reactive in templates). */
export function t(key: MessageKey, params?: Record<string, string | number>): string {
  return format(dictionaries[settings.lang][key], params);
}
