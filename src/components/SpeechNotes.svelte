<script lang="ts">
  import { Channel, invoke } from '@tauri-apps/api/core';
  import { onDestroy, tick, untrack } from 'svelte';
  import { t } from '../lib/settings.svelte';
  import { followPosition, noteWords } from '../lib/speech';
  import type { Block } from '../lib/types';
  import NoteBody from './NoteBody.svelte';

  let { blocks, slide, container }: { blocks: Block[]; slide: number; container?: HTMLElement } = $props();
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
  button:disabled { opacity: 0.5; cursor: default; }
  .hint, .error { font-size: 12px; line-height: 1.5; margin: 0 0 18px; color: var(--fg-3); }
</style>
