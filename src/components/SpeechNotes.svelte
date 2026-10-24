<script lang="ts">
  import { Channel, invoke } from '@tauri-apps/api/core';
  import { onDestroy, tick, untrack } from 'svelte';
  import { t } from '../lib/settings.svelte';
  import { finished, followPosition, noteWords } from '../lib/speech';
  import type { Block } from '../lib/types';
  import NoteBody from './NoteBody.svelte';

  let {
    blocks,
    slide,
    container,
    autoAdvance = $bindable(false),
    onfinished,
  }: {
    blocks: Block[];
    slide: number;
    container?: HTMLElement;
    autoAdvance?: boolean;
    /** The speaker has read this slide's text through (only with autoAdvance on). */
    onfinished?: () => void;
  } = $props();
  // Auto-advance fires only when the cursor really walked the text, not jumped to
  // its end, and the slide has been up for a while: the tail of the previous
  // slide's speech must not skip the next one.
  const AUTO_MIN_STEPS = 2;
  const AUTO_MIN_SHOWN_MS = 4000;
  let steps = 0;
  let shownAt = Date.now();
  type SpeechEvent = { kind: 'loading' | 'listening' | 'stopped' }
    | { kind: 'download'; file: string; bytes: number }
    | { kind: 'text'; text: string; context: number }
    | { kind: 'error'; message: string };
  let status = $state<'off' | 'loading' | 'download' | 'listening' | 'stopping'>('off');
  let position = $state(0);
  let downloaded = $state(0);
  let error = $state('');
  let context = 0;
  let alive = true;
  let generation = 0;
  let active = false;
  const script = $derived(noteWords(blocks));

  $effect(() => {
    void slide;
    void blocks;
    position = 0;
    steps = 0;
    shownAt = Date.now();
    context++;
    if (untrack(() => active)) {
      void invoke('speech_context', { context }).catch((cause: unknown) => { error = String(cause); });
    }
  });

  async function toggle() {
    if (status !== 'off') {
      status = 'stopping';
      active = false;
      try { await invoke('speech_stop'); }
      catch (cause) { error = String(cause); status = 'off'; }
      return;
    }
    error = '';
    position = 0;
    status = 'loading';
    active = true;
    const run = ++generation;
    const channel = new Channel<SpeechEvent>();
    channel.onmessage = (event) => {
      if (!alive || run !== generation) return;
      if (event.kind === 'stopped') { status = 'off'; active = false; return; }
      if (event.kind === 'error') { error = event.message; return; }
      if (!active) return;
      if (event.kind === 'text') {
        if (event.context !== context) return;
        const next = followPosition(script, event.text, position);
        if (next === position) return;
        position = next;
        steps++;
        if (autoAdvance && steps >= AUTO_MIN_STEPS && Date.now() - shownAt >= AUTO_MIN_SHOWN_MS &&
            finished(position, script.length)) onfinished?.();
        void tick().then(() => {
          if (!alive || !container) return;
          const current = container.querySelector('[data-speech-current]');
          if (!current) return;
          const bounds = container.getBoundingClientRect();
          const word = current.getBoundingClientRect();
          if (word.top > bounds.top + bounds.height * 0.65 || word.top < bounds.top + 50) {
            container.scrollBy({ top: word.top - bounds.top - bounds.height * 0.4, behavior: 'smooth' });
          }
        });
      } else if (event.kind === 'download') {
        status = 'download'; downloaded = Math.round(event.bytes / 1048576);
      } else status = event.kind;
    };
    try { await invoke('speech_start', { context, onEvent: channel }); }
    catch (cause) { if (alive) { error = String(cause); status = 'off'; active = false; } }
  }

  onDestroy(() => {
    alive = false;
    if (active || status !== 'off') void invoke('speech_stop').catch(console.error);
  });
</script>

<div class="speech-controls">
  <button class:live={status === 'listening'} aria-pressed={status === 'listening'}
    disabled={status === 'stopping' || (status === 'off' && script.length === 0)} onclick={toggle}
    title={t('speech.hint')} onkeydown={(event) => {
      if (event.key === ' ' || event.key === 'Enter') event.stopPropagation();
    }}>
    {status === 'off' ? t('speech.start') : status === 'listening' ? t('speech.stop') : status === 'stopping' ? t('speech.stopping') : t('speech.cancel')}
  </button>
  <button class="switch" role="switch" aria-checked={autoAdvance}
    class:idle={autoAdvance && status !== 'listening'}
    title={status === 'listening' ? t('speech.auto.hint') : t('speech.auto.idle')}
    onclick={() => (autoAdvance = !autoAdvance)} onkeydown={(event) => {
      if (event.key === ' ' || event.key === 'Enter') event.stopPropagation();
    }}><span class="track"><span class="knob"></span></span>{t('speech.auto')}</button>
  <span role="status">
    {#if status === 'download'}{t('speech.download', { mb: downloaded })}
    {:else if status === 'loading'}{t('speech.loading')}
    {:else if status === 'listening'}{t('speech.listening')}{/if}
  </span>
</div>
{#if status === 'off'}<p class="hint">{t('speech.hint')}</p>{/if}
{#if error}<p class="error" role="alert">{error}</p>{/if}
<NoteBody {blocks} position={status === 'listening' ? position : -1} />

<style>
  .speech-controls { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; margin-bottom: 12px; font-size: 12px; color: var(--fg-3); }
  button { border: 1px solid var(--line); border-radius: 999px; padding: 6px 12px; background: var(--surface); color: var(--fg); cursor: pointer; }
  button.live { color: var(--accent); }
  .switch { display: inline-flex; align-items: center; gap: 8px; border: 0; padding: 4px 2px; background: none; color: var(--fg-3); }
  .switch:hover, .switch[aria-checked='true'] { color: var(--fg); }
  .switch .track { position: relative; width: 30px; height: 18px; border-radius: 9px; background: var(--line); transition: background 0.2s; }
  .switch .knob { position: absolute; top: 2px; left: 2px; width: 14px; height: 14px; border-radius: 50%; background: var(--surface); box-shadow: 0 1px 2px #0003; transition: transform 0.2s; }
  .switch[aria-checked='true'] .track { background: var(--accent); }
  .switch[aria-checked='true'] .knob { transform: translateX(12px); }
  .switch.idle { opacity: 0.55; }
  button:disabled { opacity: 0.5; cursor: default; }
  .hint, .error { font-size: 12px; line-height: 1.5; margin: 0 0 18px; color: var(--fg-3); }
</style>
