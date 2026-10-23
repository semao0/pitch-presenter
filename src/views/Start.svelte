<!-- Start screen: choose a deck and notes, set the talk length, go. -->
<script lang="ts">
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import FileField from '../components/FileField.svelte';
  import Icon from '../components/Icon.svelte';
  import Preferences from '../components/Preferences.svelte';
  import { api, baseName, dirName, events } from '../lib/api';
  import { settings, t } from '../lib/settings.svelte';
  import type { Deck, Progress, RecentDeck } from '../lib/types';

  let { onopen }: { onopen: (deck: Deck) => void } = $props();

  const DECK_EXTENSIONS = ['pdf', 'pptx', 'ppt', 'odp', 'key'];
  const MAX_MINUTES = 240;

  let deck = $state<string | null>(null);
  let notes = $state<string | null>(null);
  /** The user chose the notes by hand: don't replace them when the deck changes. */
  let notesPinned = $state(false);
  let minutes = $state(0);
  let recent = $state<RecentDeck[]>([]);

  let busy = $state(false);
  let progress = $state<Progress | null>(null);
  let error = $state<string | null>(null);
  let dragging = $state(false);

  const percent = $derived(
    progress?.stage === 'rendering' && progress.total > 0 ? (progress.done / progress.total) * 100 : null,
  );

  // Backend errors come in the language they were raised in; drop a stale one.
  $effect(() => {
    void settings.lang;
    error = null;
  });

  onMount(() => {
    void api.launchInfo().then((info) => {
      deck = info.deck;
      notes = info.notes;
      minutes = info.minutes;
      recent = info.recent;
    });

    const stopOpenFiles = events.onOpenFiles((paths) => void useFiles(paths));
    let stopDrop: (() => void) | undefined;
    let disposed = false;
    void getCurrentWebview()
      .onDragDropEvent(({ payload }) => {
        if (payload.type === 'enter') dragging = true;
        else if (payload.type === 'leave') dragging = false;
        else if (payload.type === 'drop') {
          dragging = false;
          void useFiles(payload.paths);
        }
      })
      .then((unlisten) => (disposed ? unlisten() : (stopDrop = unlisten)));

    return () => {
      disposed = true;
      stopOpenFiles();
      stopDrop?.();
    };
  });

  async function useFiles(paths: string[]) {
    if (busy) return;
    const [newDeck, newNotes] = await api.classifyFiles(paths);
    if (newNotes) setNotes(newNotes);
    if (newDeck) await setDeck(newDeck, newNotes);
  }

  async function setDeck(path: string, withNotes: string | null = null) {
    error = null;
    deck = path;
    if (withNotes) {
      setNotes(withNotes);
    } else if (!notesPinned) {
      notes = recent.find((r) => r.deck === path)?.notes ?? (await api.guessNotes(path));
    }
  }

  function setNotes(path: string | null) {
    error = null;
    notes = path;
    notesPinned = path !== null;
  }

  async function pickDeck() {
    const path = await open({
      title: t('start.deck'),
      filters: [{ name: t('start.deck.filter'), extensions: DECK_EXTENSIONS }],
      defaultPath: deck ? dirName(deck) : undefined,
    });
    if (path) await setDeck(path);
  }

  async function pickNotes() {
    const path = await open({
      title: t('start.notes'),
      filters: [{ name: t('start.notes.filter'), extensions: ['toml'] }],
      defaultPath: notes ? dirName(notes) : deck ? dirName(deck) : undefined,
    });
    if (path) setNotes(path);
  }

  function setMinutes(value: number) {
    minutes = Number.isFinite(value) ? Math.min(MAX_MINUTES, Math.max(0, Math.round(value))) : 0;
  }

  async function start() {
    if (busy) return;
    if (!deck) {
      error = t('start.noDeck');
      return;
    }
    busy = true;
    error = null;
    progress = null;
    try {
      onopen(await api.openDeck(deck, notes, minutes, (p) => (progress = p)));
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
      progress = null;
    }
  }

  async function openRecent(item: RecentDeck, now: boolean) {
    notesPinned = false;
    await setDeck(item.deck, item.notes);
    notesPinned = false;
    if (now) await start();
  }

  function ago(unixSeconds: number): string {
    const relative = new Intl.RelativeTimeFormat(settings.lang, { numeric: 'auto' });
    const minutesAgo = Math.round((unixSeconds * 1000 - Date.now()) / 60_000);
    if (minutesAgo > -60) return relative.format(minutesAgo, 'minute');
    if (minutesAgo > -60 * 24) return relative.format(Math.round(minutesAgo / 60), 'hour');
    return relative.format(Math.round(minutesAgo / (60 * 24)), 'day');
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.repeat && !(e.target instanceof HTMLButtonElement)) {
      e.preventDefault();
      void start();
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="start">
  <div class="corner"><Preferences /></div>

  <div class="layout" class:has-recent={recent.length > 0}>
    <header>
      <img class="logo" src="/icon.svg" alt="" width="56" height="56" />
      <h1>Pitch Presenter</h1>
      <p>{t('app.tagline')}</p>
    </header>

    <section class="card" aria-busy={busy}>
      <FileField
        label={t('start.deck')}
        icon="file"
        path={deck}
        placeholder={t('start.deck.placeholder')}
        hint={t('start.deck.hint')}
        disabled={busy}
        onpick={pickDeck}
      />
      <FileField
        label={t('start.notes')}
        icon="notes"
        path={notes}
        placeholder={t('start.notes.placeholder')}
        hint={t('start.notes.hint')}
        disabled={busy}
        onpick={pickNotes}
        onclear={() => setNotes(null)}
      />

      <div class="minutes">
        <label class="label" for="minutes">{t('start.minutes')}</label>
        <div class="stepper">
          <button
            class="icon-btn"
            disabled={busy || minutes <= 0}
            onclick={() => setMinutes(minutes - 1)}
            aria-label={t('start.less')}
          >
            <Icon name="minus" size={16} />
          </button>
          <input
            id="minutes"
            class="tabular"
            type="number"
            min="0"
            max={MAX_MINUTES}
            disabled={busy}
            value={minutes}
            onchange={(e) => setMinutes(e.currentTarget.valueAsNumber)}
          />
          <button
            class="icon-btn"
            disabled={busy || minutes >= MAX_MINUTES}
            onclick={() => setMinutes(minutes + 1)}
            aria-label={t('start.more')}
          >
            <Icon name="plus" size={16} />
          </button>
        </div>
        <span class="hint">{minutes > 0 ? t('start.minutes.unit') : t('start.minutes.none')}</span>
      </div>

      <footer>
        <div class="status" role="status">
          {#if busy}
            <span>
              {#if progress?.stage === 'converting'}
                {t('start.converting')}
              {:else if progress?.stage === 'rendering'}
                {t('start.rendering')} · <span class="tabular">{progress.done} / {progress.total}</span>
              {:else}
                {t('start.opening')}
              {/if}
            </span>
            <div class="bar" class:indeterminate={percent === null}>
              <div style:width={percent === null ? null : `${percent}%`}></div>
            </div>
          {:else if error}
            <span class="error">{error}</span>
          {/if}
        </div>
        <button class="btn primary go" disabled={busy || !deck} onclick={start}>
          <span class="dot"></span>
          {t('start.go')}
        </button>
      </footer>
    </section>

    {#if recent.length > 0}
      <aside class="recent">
        <h2 class="label">{t('start.recent')}</h2>
        <ul class="thin-scroll">
          {#each recent as item (item.deck)}
            <li>
              <button
                class:active={item.deck === deck}
                disabled={busy}
                onclick={() => openRecent(item, false)}
                ondblclick={() => openRecent(item, true)}
                title="{item.deck}&#10;{t('start.recent.hint')}"
              >
                <span class="name">{baseName(item.deck)}</span>
                <span class="meta">
                  {#if item.notes}<Icon name="notes" size={13} />{/if}
                  {ago(item.openedAt)}
                </span>
              </button>
            </li>
          {/each}
        </ul>
      </aside>
    {/if}
  </div>

  {#if dragging}
    <div class="drop">
      <Icon name="upload" size={32} />
      <span>{t('start.drop')}</span>
      <small>{t('start.drop.hint')}</small>
    </div>
  {/if}
</div>

<style>
  .start {
    position: relative;
    height: 100%;
    display: grid;
    place-items: center;
    padding: 32px;
    overflow-y: auto;
  }

  .corner {
    position: absolute;
    top: 14px;
    right: 16px;
  }

  .layout {
    display: grid;
    grid-template-columns: minmax(0, 500px);
    grid-template-areas:
      'header'
      'card';
    gap: 28px 32px;
  }

  .layout.has-recent {
    grid-template-columns: minmax(0, 500px) minmax(0, 260px);
    grid-template-areas:
      'header .'
      'card recent';
  }

  header {
    grid-area: header;
    display: grid;
    justify-items: start;
    gap: 6px;
  }

  .logo {
    margin-bottom: 14px;
  }

  h1 {
    margin: 0;
    font-size: 30px;
    font-weight: 700;
    letter-spacing: -0.025em;
  }

  header p {
    margin: 0;
    max-width: 30em;
    color: var(--fg-3);
    font-size: 15px;
  }

  .card {
    grid-area: card;
    display: grid;
    gap: 22px;
    padding: 28px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius-l);
    box-shadow: var(--shadow);
  }

  .minutes {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: center;
    gap: 8px 12px;
  }

  .minutes label {
    grid-column: 1 / -1;
  }

  .stepper {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--line-strong);
    border-radius: 999px;
  }

  .stepper input {
    width: 52px;
    padding: 6px 0;
    text-align: center;
    font-weight: 650;
    background: none;
    border: 0;
    outline: none;
    appearance: textfield;
    -moz-appearance: textfield;
  }

  .stepper input::-webkit-inner-spin-button,
  .stepper input::-webkit-outer-spin-button {
    -webkit-appearance: none;
  }

  .stepper .icon-btn:disabled {
    opacity: 0.35;
    cursor: default;
    background: none;
  }

  .hint {
    color: var(--fg-3);
    font-size: 13px;
  }

  footer {
    display: flex;
    align-items: center;
    gap: 16px;
    padding-top: 6px;
    min-height: 44px;
  }

  .status {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 8px;
    font-size: 13px;
    color: var(--fg-2);
  }

  .error {
    color: var(--bad);
    user-select: text;
    -webkit-user-select: text;
    overflow-wrap: anywhere;
  }

  .bar {
    height: 3px;
    border-radius: 2px;
    background: var(--line);
    overflow: hidden;
  }

  .bar div {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.2s var(--ease);
  }

  .bar.indeterminate div {
    width: 35%;
    animation: slide 1.1s var(--ease) infinite;
  }

  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(300%);
    }
  }

  .go {
    height: 44px;
    padding: 0 24px 0 20px;
    font-size: 15px;
  }

  .go .dot {
    width: 10px;
    height: 10px;
  }

  .recent {
    grid-area: recent;
    display: flex;
    flex-direction: column;
    min-height: 0;
    max-height: 420px;
  }

  h2 {
    margin: 0 0 10px 12px;
  }

  .recent ul {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
  }

  .recent li button {
    width: 100%;
    display: grid;
    gap: 3px;
    padding: 10px 12px;
    text-align: left;
    border-radius: var(--radius);
    border: 1px solid transparent;
    transition:
      background 0.15s,
      border-color 0.15s;
  }

  .recent li button:hover:not(:disabled) {
    background: var(--surface);
  }

  .recent li button.active {
    background: var(--surface);
    border-color: var(--line);
  }

  .recent .name {
    font-weight: 550;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .recent .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--fg-3);
  }

  .drop {
    position: fixed;
    inset: 12px;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 10px;
    border: 2px dashed var(--accent);
    border-radius: var(--radius-l);
    background: var(--backdrop);
    color: var(--accent);
    font-size: 18px;
    font-weight: 550;
    pointer-events: none;
    animation: fade-in 0.12s ease-out;
  }

  .drop small {
    color: var(--fg-3);
    font-size: 13px;
    font-weight: 400;
  }

  @media (max-width: 900px) {
    .layout.has-recent {
      grid-template-columns: minmax(0, 500px);
      grid-template-areas:
        'header'
        'card';
    }
    .recent {
      display: none;
    }
  }
</style>
