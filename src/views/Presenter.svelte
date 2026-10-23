<!-- Presenter view: current and next slide, speaker notes, talk timer. -->
<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount, untrack } from 'svelte';
  import Help from '../components/Help.svelte';
  import Icon from '../components/Icon.svelte';
  import NoteBody from '../components/NoteBody.svelte';
  import Overview from '../components/Overview.svelte';
  import { api, events, slideUrl } from '../lib/api';
  import { computePlan, formatDuration, pace } from '../lib/plan';
  import { SlidePreloader } from '../lib/preload';
  import { prefs } from '../lib/storage';
  import { TalkTimer } from '../lib/timer.svelte';
  import type { Deck, Nav, ViewState } from '../lib/types';

  let { deck, onclose }: { deck: Deck; onclose: () => void } = $props();

  const FONT_MIN = 16;
  const FONT_MAX = 72;
  const SPLIT_MIN = 30;
  const SPLIT_MAX = 75;
  const GOTO_TIMEOUT_MS = 2500;

  // The parent re-creates this component for every new deck, so the id is fixed.
  const deckId = untrack(() => deck.id);
  const timer = new TalkTimer(deckId);
  const preloader = new SlidePreloader();

  let view = $state<ViewState>({ deckId, count: untrack(() => deck.count), slide: 1, blank: false });
  let audienceOpen = $state(false);
  let fontSize = $state(prefs.get('font', 30));
  let split = $state(prefs.get('split', 58));
  let showHelp = $state(false);
  let showWarnings = $state(false);
  let overview = $state<{ selected: number } | null>(null);
  let gotoBuffer = $state('');
  let gotoTimer: ReturnType<typeof setTimeout> | undefined;
  let notesEl = $state<HTMLElement>();

  const plan = $derived(computePlan(deck));
  const note = $derived(deck.notes.slides[view.slide]);
  const nextNote = $derived(deck.notes.slides[view.slide + 1]);
  const isLast = $derived(view.slide >= deck.count);
  const budget = $derived(plan.budget[view.slide] ?? 0);
  const spentHere = $derived(timer.spentOn(view.slide));
  const remaining = $derived(deck.minutes > 0 ? deck.minutes * 60_000 - timer.elapsed : null);
  const currentPace = $derived(timer.pristine ? null : pace(plan, view.slide, timer.elapsed, spentHere));
  const clock = $derived(new Date(timer.now).toLocaleTimeString('ru', { hour: '2-digit', minute: '2-digit' }));
  const warnings = $derived(deck.notes.warnings);

  // Slide sizes follow the stage box. Measured with a ResizeObserver (via
  // bind:client*) rather than container query units, which WebKitGTK does not
  // recompute when the window is resized.
  const STAGE_PAD_X = 32;
  const STAGE_PAD_Y = 44;
  let stageWidth = $state(0);
  let stageHeight = $state(0);
  const innerWidth = $derived(Math.max(0, stageWidth - STAGE_PAD_X));
  const innerHeight = $derived(Math.max(0, stageHeight - STAGE_PAD_Y));
  const currentWidth = $derived(Math.min(innerWidth, innerHeight * 0.64 * deck.aspect));
  const nextWidth = $derived(Math.min(innerWidth * 0.46, innerHeight * 0.22 * deck.aspect));

  // ---- show state ----------------------------------------------------------
  // Navigation is optimistic: the presenter updates instantly and tells the
  // backend the *absolute* target, so repeated or racing requests are harmless.
  // Echoes are ignored while requests are in flight; afterwards we re-sync, in
  // case the audience window navigated meanwhile.

  let inFlight = 0;

  function send(nav: Nav) {
    inFlight++;
    void api
      .navigate(nav)
      .catch((e: unknown) => console.error('navigate failed', e))
      .finally(() => {
        if (--inFlight === 0) void api.currentView().then((v) => v && applyView(v));
      });
  }

  function applyView(next: ViewState) {
    if (next.deckId !== deckId) return;
    if (next.slide !== view.slide) timer.tick(view.slide);
    view = next;
  }

  function go(slide: number) {
    const target = Math.min(Math.max(1, slide), deck.count);
    if (target === view.slide) return;
    timer.tick(view.slide);
    // The timer starts by itself when the speaker leaves the title slide.
    if (timer.pristine) timer.toggle();
    view = { ...view, slide: target };
    send({ type: 'goto', slide: target });
  }

  function setBlank(blank: boolean) {
    view = { ...view, blank };
    send({ type: 'setBlank', blank });
  }

  // ---- keyboard --------------------------------------------------------------

  function handleKey(key: string): boolean {
    if (showHelp) {
      showHelp = false;
      return true;
    }
    if (overview) return overviewKey(key);
    if (/^\d$/.test(key)) {
      setGoto((gotoBuffer + key).slice(-4));
      return true;
    }
    if (gotoBuffer) {
      if (key === 'Enter') {
        go(Number(gotoBuffer));
        setGoto('');
        return true;
      }
      if (key === 'Backspace') {
        setGoto(gotoBuffer.slice(0, -1));
        return true;
      }
      if (key === 'Escape') {
        setGoto('');
        return true;
      }
    }
    switch (key) {
      case 'ArrowRight':
      case 'PageDown':
      case ' ':
      case 'Enter':
      case 'n':
        go(view.slide + 1);
        return true;
      case 'ArrowLeft':
      case 'PageUp':
      case 'Backspace':
      case 'p':
        go(view.slide - 1);
        return true;
      case 'Home':
        go(1);
        return true;
      case 'End':
        go(deck.count);
        return true;
      case 'ArrowDown':
        notesEl?.scrollBy({ top: fontSize * 3, behavior: 'smooth' });
        return true;
      case 'ArrowUp':
        notesEl?.scrollBy({ top: -fontSize * 3, behavior: 'smooth' });
        return true;
      case 'b':
      case 'B':
      case '.':
        setBlank(!view.blank);
        return true;
      case 't':
      case 'T':
        timer.toggle();
        return true;
      case 'R':
        timer.reset();
        return true;
      case '+':
      case '=':
        setFont(fontSize + 2);
        return true;
      case '-':
      case '_':
        setFont(fontSize - 2);
        return true;
      case 'a':
      case 'A':
        void api.openAudience();
        return true;
      case 'g':
      case 'G':
        overview = { selected: view.slide };
        return true;
      case 'f':
      case 'F':
        void toggleFullscreen();
        return true;
      case '?':
      case 'F1':
        showHelp = true;
        return true;
      case 'Escape':
        showWarnings = false;
        return true;
    }
    return false;
  }

  function overviewKey(key: string): boolean {
    if (!overview) return false;
    const columns = overviewColumns();
    const move: Record<string, number> = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: columns, ArrowUp: -columns };
    if (key in move) {
      overview.selected = Math.min(Math.max(1, overview.selected + move[key]!), deck.count);
    } else if (key === 'Enter' || key === ' ') {
      go(overview.selected);
      overview = null;
    } else if (key === 'Escape' || key === 'g' || key === 'G') {
      overview = null;
    }
    return true;
  }

  /** Columns in the overview grid, for ↑/↓ navigation. */
  function overviewColumns(): number {
    const grid = document.querySelector<HTMLElement>('[role="listbox"]');
    return grid ? getComputedStyle(grid).gridTemplateColumns.split(' ').length : 4;
  }

  function setGoto(value: string) {
    gotoBuffer = value;
    clearTimeout(gotoTimer);
    if (value) gotoTimer = setTimeout(() => (gotoBuffer = ''), GOTO_TIMEOUT_MS);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    if (handleKey(e.key)) e.preventDefault();
  }

  // ---- layout ------------------------------------------------------------------

  function setFont(px: number) {
    fontSize = Math.min(FONT_MAX, Math.max(FONT_MIN, px));
    prefs.set('font', fontSize);
  }

  function startResize(e: PointerEvent) {
    const handle = e.currentTarget as HTMLElement;
    handle.setPointerCapture(e.pointerId);
    const move = (ev: PointerEvent) => {
      split = Math.min(SPLIT_MAX, Math.max(SPLIT_MIN, (ev.clientX / window.innerWidth) * 100));
    };
    const up = () => {
      handle.removeEventListener('pointermove', move);
      prefs.set('split', split);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('lostpointercapture', up, { once: true });
  }

  async function toggleFullscreen() {
    const win = getCurrentWindow();
    await win.setFullscreen(!(await win.isFullscreen()));
  }

  async function close() {
    // Reopening the same deck later resumes the timer; don't count the break.
    if (timer.running) timer.toggle();
    await api.closeDeck();
    onclose();
  }

  // ---- lifecycle -------------------------------------------------------------

  onMount(() => {
    void api.currentView().then((v) => v && applyView(v));
    void api.audienceOpen().then((open) => (audienceOpen = open));
    const stop = [
      events.onView((v) => inFlight === 0 && applyView(v)),
      events.onAudience((open) => (audienceOpen = open)),
      events.onRemoteKey((key) => handleKey(key)),
    ];
    const tick = setInterval(() => timer.tick(view.slide), 250);
    return () => {
      stop.forEach((fn) => fn());
      clearInterval(tick);
      clearTimeout(gotoTimer);
    };
  });

  $effect(() => preloader.setDeck(deck.id, deck.count));
  $effect(() => preloader.focus(view.slide, deck.count));

  // A new slide starts with its notes scrolled to the top.
  $effect(() => {
    void view.slide;
    notesEl?.scrollTo({ top: 0 });
  });

  $effect(() => {
    void getCurrentWindow().setTitle(`${view.slide}/${deck.count} · ${deck.title}`);
  });
</script>

<svelte:window {onkeydown} />

<div class="presenter" style:--note-size="{fontSize}px" style:--aspect={deck.aspect}>
  <header>
    <div class="position tabular" title="Слайд">
      {view.slide}<span>/ {deck.count}</span>
    </div>
    <div class="title">
      <span>{deck.title}</span>
      {#if deck.notesFile}<small>{deck.notesFile}</small>{/if}
    </div>

    {#if warnings.length > 0}
      <div class="warnings">
        <button class="icon-btn warn" onclick={() => (showWarnings = !showWarnings)} title="Предупреждения">
          <Icon name="warning" />
          <span class="tabular">{warnings.length}</span>
        </button>
        {#if showWarnings}
          <ul class="popover" aria-label="Предупреждения">
            {#each warnings as warning (warning)}<li>{warning}</li>{/each}
          </ul>
        {/if}
      </div>
    {/if}

    <button class="timer tabular" onclick={() => timer.toggle()} title="Таймер: старт / пауза — T, сброс — Shift R">
      <span class="elapsed" class:paused={!timer.running}>{formatDuration(timer.elapsed)}</span>
      {#if remaining !== null}
        <span class="remaining" class:warn={remaining >= 0 && remaining < 60_000} class:bad={remaining < 0}>
          {remaining >= 0 ? '−' : '+'}{formatDuration(remaining)}
        </span>
      {/if}
    </button>

    {#if currentPace}
      <span class="pace tabular {currentPace.status}" title="Темп относительно плана">
        {#if currentPace.status === 'onTime'}по плану
        {:else if currentPace.status === 'ahead'}запас {formatDuration(currentPace.diff)}
        {:else}отстаём {formatDuration(currentPace.diff)}{/if}
      </span>
    {/if}

    <span class="clock tabular" title="Текущее время">{clock}</span>

    <button class="audience" class:on={audienceOpen} onclick={() => api.openAudience()} title="Окно для показа — A">
      <i></i>{audienceOpen ? 'Показ открыт' : 'Показ'}
    </button>
    <div class="tools">
      <button class="icon-btn" onclick={() => (overview = { selected: view.slide })} title="Все слайды — G">
        <Icon name="grid" />
      </button>
      <button class="icon-btn" onclick={() => (showHelp = true)} title="Клавиши — ?">
        <Icon name="help" />
      </button>
      <button class="icon-btn" onclick={close} title="Закрыть презентацию">
        <Icon name="exit" />
      </button>
    </div>
    <div class="progress" style:width="{(view.slide / deck.count) * 100}%"></div>
  </header>

  {#if deck.notesError}
    <div class="banner" role="alert">
      <Icon name="warning" size={16} />
      <span><b>Текст не обновлён.</b> {deck.notesError}</span>
    </div>
  {/if}

  <main style:--split="{split}%">
    <section class="stage" aria-label="Слайды" bind:clientWidth={stageWidth} bind:clientHeight={stageHeight}>
      <button
        class="current"
        class:blank={view.blank}
        style:width="{currentWidth}px"
        onclick={() => go(view.slide + 1)}
        oncontextmenu={(e) => {
          e.preventDefault();
          go(view.slide - 1);
        }}
        aria-label="Следующий слайд"
      >
        <img src={slideUrl(deck.id, view.slide)} alt="Слайд {view.slide}" draggable="false" />
        {#if view.blank}<span class="badge">Экран скрыт · B</span>{/if}
      </button>

      <div
        class="slide-time tabular"
        style:width="{currentWidth}px"
        class:hidden={!budget}
        class:warn={spentHere > budget * 0.8 && spentHere <= budget}
        class:over={spentHere > budget}
        title="Время на этом слайде / по плану"
      >
        <div class="track"><div class="fill" style:width="{Math.min(100, (spentHere / budget) * 100 || 0)}%"></div></div>
        <span>
          {spentHere > budget
            ? `+${formatDuration(spentHere - budget)} сверх ${formatDuration(budget)}`
            : `${formatDuration(spentHere)} / ${formatDuration(budget)}`}
        </span>
      </div>

      <div class="next-label">{isLast ? 'Конец презентации' : 'Далее'}</div>
      {#if !isLast}
        <button class="next" style:width="{nextWidth}px" onclick={() => go(view.slide + 1)} aria-label="Перейти к следующему слайду">
          <img src={slideUrl(deck.id, view.slide + 1)} alt="" draggable="false" />
        </button>
      {/if}
    </section>

    <div
      class="splitter"
      role="separator"
      aria-orientation="vertical"
      aria-valuenow={Math.round(split)}
      onpointerdown={startResize}
      ondblclick={() => {
        split = 58;
        prefs.set('split', split);
      }}
      title="Перетащите, чтобы изменить ширину · двойной клик — сбросить"
    ></div>

    <section class="notes thin-scroll" bind:this={notesEl} aria-label="Текст спикера">
      <div class="note-title">
        {note?.title ?? `Слайд ${view.slide}`}
        {#if note?.reserve}<span class="reserve">резерв</span>{/if}
        {#if note?.seconds}<span class="limit tabular"><Icon name="clock" size={13} />{formatDuration(note.seconds * 1000)}</span>{/if}
      </div>
      <NoteBody blocks={note?.blocks ?? []} />
      {#if !isLast}
        <div class="upnext">Дальше — {nextNote?.title ?? `слайд ${view.slide + 1}`}</div>
      {/if}
    </section>
  </main>

  {#if gotoBuffer}
    <div class="goto tabular" aria-live="polite">
      <span>{gotoBuffer}</span><small>Enter — перейти</small>
    </div>
  {/if}
  {#if overview}
    <Overview
      {deck}
      current={view.slide}
      selected={overview.selected}
      onpick={(n) => {
        go(n);
        overview = null;
      }}
      onclose={() => (overview = null)}
    />
  {/if}
  {#if showHelp}<Help onclose={() => (showHelp = false)} />{/if}
</div>

<style>
  .presenter {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  /* ---- header ---- */

  header {
    position: relative;
    display: flex;
    align-items: center;
    gap: 22px;
    padding: 12px 18px 12px 26px;
    border-bottom: 2px solid var(--line);
  }

  .progress {
    position: absolute;
    left: 0;
    bottom: -2px;
    height: 2px;
    background: var(--accent);
    transition: width 0.25s var(--ease);
  }

  .position {
    font-size: 22px;
    font-weight: 650;
    letter-spacing: 0.01em;
  }

  .position span {
    margin-left: 4px;
    color: var(--fg-4);
    font-weight: 400;
  }

  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 10px;
    color: var(--fg-2);
    font-size: 14px;
    white-space: nowrap;
  }

  .title span,
  .title small {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .title small {
    color: var(--fg-4);
    font-size: 12px;
  }

  .warnings {
    position: relative;
  }

  .icon-btn.warn {
    width: auto;
    gap: 6px;
    padding: 0 8px;
    color: var(--warn);
    display: inline-flex;
    align-items: center;
  }

  .popover {
    position: absolute;
    right: 0;
    top: calc(100% + 8px);
    z-index: 10;
    width: 380px;
    margin: 0;
    padding: 12px 14px 12px 30px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    font-size: 13px;
    line-height: 1.5;
    color: var(--fg-2);
    user-select: text;
    -webkit-user-select: text;
  }

  .timer {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 4px 8px;
    border-radius: var(--radius-s);
  }

  .timer:hover {
    background: var(--surface-2);
  }

  .elapsed {
    font-size: 26px;
    font-weight: 650;
  }

  .elapsed.paused {
    color: var(--fg-4);
  }

  .remaining {
    font-size: 14px;
    color: var(--fg-3);
  }

  .remaining.warn {
    color: var(--warn);
  }

  .remaining.bad {
    color: var(--bad);
  }

  .pace {
    font-size: 14px;
    color: var(--fg-3);
  }

  .pace.ahead {
    color: var(--ok);
  }

  .pace.behind {
    color: var(--warn);
  }

  .pace.late {
    color: var(--bad);
  }

  .clock {
    font-size: 14px;
    color: var(--fg-3);
  }

  .audience {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 14px;
    font-size: 14px;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    white-space: nowrap;
    transition: border-color 0.15s;
  }

  .audience:hover {
    border-color: var(--line-strong);
  }

  .audience i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--fg-4);
    transition:
      background 0.2s,
      box-shadow 0.2s;
  }

  .audience.on i {
    background: var(--ok);
    box-shadow: 0 0 0 3px rgb(76 195 138 / 20%);
  }

  .tools {
    display: flex;
    gap: 2px;
  }

  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 26px;
    background: var(--bad-soft);
    color: var(--bad);
    font-size: 13px;
    user-select: text;
    -webkit-user-select: text;
  }

  .banner b {
    font-weight: 600;
  }

  /* ---- workspace ---- */

  main {
    flex: 1;
    display: grid;
    grid-template-columns: var(--split) 10px minmax(0, 1fr);
    min-height: 0;
  }

  .stage {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 22px 6px 22px 26px;
    overflow: hidden;
  }

  .current,
  .next {
    position: relative;
    display: block;
    flex: none;
    aspect-ratio: var(--aspect);
    border-radius: var(--radius-s);
    overflow: hidden;
    background: var(--surface);
  }

  .next {
    opacity: 0.6;
    transition: opacity 0.15s;
  }

  .next:hover {
    opacity: 0.85;
  }

  /* Big click targets: a focus ring around the slide would only distract. */
  .current:focus-visible,
  .next:focus-visible {
    outline: none;
  }

  .current img,
  .next img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .current.blank img {
    opacity: 0.15;
  }

  .badge {
    position: absolute;
    left: 14px;
    top: 12px;
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--warn-soft);
    color: var(--warn);
    font-size: 13px;
    font-weight: 500;
  }

  .slide-time {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-top: 12px;
    font-size: 14px;
    color: var(--fg-3);
  }

  .slide-time.hidden {
    visibility: hidden;
  }

  .track {
    flex: 1;
    height: 6px;
    border-radius: 3px;
    background: var(--line);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    border-radius: 3px;
    background: var(--accent);
    transition:
      width 0.25s linear,
      background 0.3s;
  }

  .slide-time.warn .fill {
    background: var(--warn);
  }

  .slide-time.warn span {
    color: var(--warn);
  }

  .slide-time.over .fill {
    background: var(--bad);
  }

  .slide-time.over span {
    color: var(--bad);
    font-weight: 600;
  }

  .slide-time.over .track {
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.45;
    }
  }

  .next-label {
    margin: 20px 0 8px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-4);
  }

  .splitter {
    position: relative;
    cursor: col-resize;
    touch-action: none;
  }

  .splitter::after {
    content: '';
    position: absolute;
    top: 22px;
    bottom: 22px;
    left: 4px;
    width: 2px;
    border-radius: 1px;
    background: transparent;
    transition: background 0.15s;
  }

  .splitter:hover::after {
    background: var(--line-strong);
  }

  .notes {
    overflow-y: auto;
    min-height: 0;
    /* Room to scroll the last lines up to eye level. */
    padding: 22px 40px 40vh 30px;
  }

  .note-title {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 2px 0 18px;
    font-size: 15px;
    font-weight: 550;
    color: var(--accent);
    letter-spacing: 0.01em;
  }

  .reserve {
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--warn-soft);
    color: var(--warn);
    font-size: 12px;
    font-weight: 500;
  }

  .limit {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--fg-3);
    font-size: 13px;
    font-weight: 400;
  }

  .upnext {
    margin-top: 28px;
    font-size: 15px;
    color: var(--fg-3);
  }

  /* ---- overlays ---- */

  .goto {
    position: fixed;
    inset: 0;
    z-index: 25;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 6px;
    background: rgb(6 7 9 / 60%);
    pointer-events: none;
  }

  .goto span {
    font-size: 72px;
    font-weight: 650;
  }

  .goto small {
    color: var(--fg-3);
    font-size: 14px;
  }

  @media (max-width: 960px) {
    main {
      grid-template-columns: minmax(0, 1fr);
    }
    .stage,
    .splitter,
    .title,
    .clock {
      display: none;
    }
  }
</style>
