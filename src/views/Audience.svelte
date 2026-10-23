<!-- Audience window: the current slide, nothing else. Shared to a call or shown
     on a projector. Keys are forwarded to the presenter, so the speaker can
     control the talk whichever window has focus. -->
<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import { api, events, slideUrl } from '../lib/api';
  import { shortcutKey } from '../lib/keys';
  import { t } from '../lib/settings.svelte';
  import { SlidePreloader } from '../lib/preload';
  import type { ViewState } from '../lib/types';

  const CURSOR_IDLE_MS = 1500;
  const HINT_MS = 4000;
  /** Delay before a click navigates, so a double click (fullscreen) doesn't. */
  const CLICK_DELAY_MS = 220;

  const win = getCurrentWindow();
  const preloader = new SlidePreloader();

  let view = $state<ViewState | null>(null);
  // Two stacked layers: the next slide loads in the hidden one and is swapped
  // in once ready, so there is never a blank frame between slides.
  let layers = $state<[string, string]>(['', '']);
  let front = $state(0);
  let idle = $state(false);
  let hint = $state(true);

  let idleTimer: ReturnType<typeof setTimeout> | undefined;
  let clickTimer: ReturnType<typeof setTimeout> | undefined;

  function show(next: ViewState) {
    view = next;
    preloader.setDeck(next.deckId, next.count);
    preloader.focus(next.slide, next.count);
    const url = slideUrl(next.deckId, next.slide);
    if (layers[front] === url) return;
    const back = 1 - front;
    if (layers[back] === url) front = back;
    else layers[back] = url;
    void win.setTitle(`${t('audience.title')} · ${next.slide}/${next.count}`);
  }

  function onLayerLoad(index: number, e: Event) {
    // Only swap if this layer still holds the slide we want.
    if (view && (e.currentTarget as HTMLImageElement).src === slideUrl(view.deckId, view.slide)) front = index;
  }

  async function toggleFullscreen() {
    await win.setFullscreen(!(await win.isFullscreen()));
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    e.preventDefault();
    const key = shortcutKey(e);
    if (key === 'f' || key === 'F') {
      void toggleFullscreen();
    } else if (key === 'Escape') {
      void win.setFullscreen(false);
    } else {
      void events.sendRemoteKey(key);
    }
  }

  function onclick(e: MouseEvent) {
    clearTimeout(clickTimer);
    if (e.detail === 1) clickTimer = setTimeout(() => void api.navigate({ type: 'next' }), CLICK_DELAY_MS);
  }

  function oncontextmenu(e: MouseEvent) {
    e.preventDefault();
    void api.navigate({ type: 'prev' });
  }

  function onmousemove() {
    idle = false;
    clearTimeout(idleTimer);
    idleTimer = setTimeout(() => (idle = true), CURSOR_IDLE_MS);
  }

  onMount(() => {
    void api.currentView().then((v) => v && show(v));
    const stop = events.onView(show);
    const hintTimer = setTimeout(() => (hint = false), HINT_MS);
    onmousemove();
    return () => {
      stop();
      clearTimeout(hintTimer);
      clearTimeout(idleTimer);
      clearTimeout(clickTimer);
    };
  });
</script>

<svelte:window {onkeydown} />

<div
  class="audience"
  class:idle
  role="presentation"
  {onclick}
  {oncontextmenu}
  ondblclick={toggleFullscreen}
  {onmousemove}
>
  {#each layers as src, i (i)}
    {#if src}
      <img {src} alt="" class:front={front === i} draggable="false" onload={(e) => onLayerLoad(i, e)} />
    {/if}
  {/each}
  <div class="blank" class:on={view?.blank}></div>
  {#if !view}
    <p class="empty">{t('audience.empty')}</p>
  {/if}
  <div class="hint" class:gone={!hint}>{t('audience.hint')}</div>
</div>

<style>
  .audience {
    position: fixed;
    inset: 0;
    background: #000;
    overflow: hidden;
  }

  .audience.idle {
    cursor: none;
  }

  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
    visibility: hidden;
  }

  img.front {
    visibility: visible;
  }

  .blank {
    position: absolute;
    inset: 0;
    background: #000;
    opacity: 0;
    transition: opacity 0.2s;
    pointer-events: none;
  }

  .blank.on {
    opacity: 1;
  }

  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    color: #444;
  }

  .hint {
    position: absolute;
    left: 50%;
    bottom: 14px;
    transform: translateX(-50%);
    padding: 7px 14px;
    border-radius: 8px;
    background: rgb(20 20 20 / 80%);
    color: #999;
    font-size: 13px;
    white-space: nowrap;
    pointer-events: none;
    transition: opacity 0.6s;
  }

  .hint.gone {
    opacity: 0;
  }
</style>
