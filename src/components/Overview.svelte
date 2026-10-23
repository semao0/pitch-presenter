<!-- Grid of all slides for jumping around. Keyboard selection is driven by the
     presenter (the audience window forwards keys there too). -->
<script lang="ts">
  import { slideUrl } from '../lib/api';
  import { t } from '../lib/settings.svelte';
  import type { Deck } from '../lib/types';

  interface Props {
    deck: Deck;
    current: number;
    selected: number;
    onpick: (slide: number) => void;
    onclose: () => void;
  }

  let { deck, current, selected, onpick, onclose }: Props = $props();

  const slides = $derived(Array.from({ length: deck.count }, (_, i) => i + 1));

  // Keep the keyboard selection in view.
  let grid = $state<HTMLElement>();
  $effect(() => {
    grid?.querySelector(`[data-slide="${selected}"]`)?.scrollIntoView({ block: 'nearest' });
  });
</script>

<div class="backdrop" role="presentation" onclick={onclose}>
  <div
    class="grid thin-scroll"
    style:--aspect={deck.aspect}
    bind:this={grid}
    role="listbox"
    tabindex="-1"
    aria-label={t('overview.label')}
    onclick={(e) => e.stopPropagation()}
    onkeydown={() => {}}
  >
    {#each slides as n (n)}
      {@const note = deck.notes.slides[n]}
      <button
        data-slide={n}
        class:current={n === current}
        class:selected={n === selected}
        role="option"
        aria-selected={n === selected}
        onclick={() => onpick(n)}
      >
        <img src={slideUrl(deck.id, n, true)} alt="" loading="lazy" decoding="async" />
        <span class="caption">
          <span class="n tabular">{n}</span>
          <span class="title">{note?.title ?? ''}</span>
          {#if note?.reserve}<span class="reserve">{t('presenter.reserve')}</span>{/if}
        </span>
      </button>
    {/each}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: grid;
    padding: 40px;
    background: var(--backdrop);
    backdrop-filter: blur(8px);
    animation: fade-in 0.14s ease-out;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    align-content: start;
    gap: 24px 20px;
    max-width: 1500px;
    width: 100%;
    margin: 0 auto;
    overflow-y: auto;
    padding: 6px;
    outline: none;
  }

  button {
    display: grid;
    gap: 8px;
    text-align: left;
  }

  img {
    width: 100%;
    aspect-ratio: var(--aspect);
    object-fit: contain;
    background: var(--surface);
    border-radius: var(--radius-s);
    box-shadow: var(--slide-shadow);
    outline: 2px solid transparent;
    outline-offset: 4px;
    transition: outline-color 0.12s;
  }

  button:hover img {
    outline-color: var(--line-strong);
  }

  .current img {
    outline-color: var(--fg-3);
  }

  .selected img {
    outline-color: var(--accent);
  }

  .caption {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    font-size: 13px;
  }

  .n {
    color: var(--fg);
    font-weight: 650;
  }

  .selected .n {
    color: var(--accent);
  }

  .title {
    color: var(--fg-3);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .reserve {
    margin-left: auto;
    color: var(--warn);
    font-size: 11px;
  }
</style>
