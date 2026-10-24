<!-- Speaker note text: paragraphs, bullet lists and cues (remarks to self). -->
<script lang="ts">
  import { t } from '../lib/settings.svelte';
  import type { Block } from '../lib/types';
  import Inlines from './Inlines.svelte';
  import { blockOffsets, inlineWords } from '../lib/speech';

  let { blocks, position = -1 }: { blocks: Block[]; position?: number } = $props();
  const offsets = $derived(blockOffsets(blocks));
</script>

<div class="note">
  {#each blocks as block, i (i)}
    {#if block.kind === 'paragraph'}
      <p><Inlines nodes={block.content} offset={offsets[i]!} {position} /></p>
    {:else if block.kind === 'list'}
      <ul>
        {#each block.content as item, j (j)}
          <li><Inlines nodes={item} offset={offsets[i]! + block.content.slice(0, j).flatMap(inlineWords).length} {position} /></li>
        {/each}
      </ul>
    {:else}
      <p class="cue"><Inlines nodes={block.content} /></p>
    {/if}
  {:else}
    <p class="empty">{t('presenter.noText')}</p>
  {/each}
</div>

<style>
  .note {
    font-size: var(--note-size, 30px);
    line-height: 1.5;
    max-width: 42ch;
    letter-spacing: -0.005em;
    text-wrap: pretty;
  }

  p,
  ul {
    margin: 0 0 0.7em;
  }

  ul {
    padding-left: 1.1em;
  }

  li::marker {
    color: var(--accent);
  }

  .note :global(strong) {
    font-weight: 700;
  }

  /* ==highlight== reads like a red pen mark. */
  .note :global(mark) {
    background: none;
    color: var(--accent);
    font-weight: 600;
  }

  .note :global(code) {
    font: 0.85em var(--mono);
    background: var(--surface-2);
    padding: 0 0.3em;
    border-radius: 4px;
  }

  .cue {
    font-size: 0.58em;
    color: var(--fg-3);
  }

  .cue::before {
    content: '↳ ';
    color: var(--fg-4);
  }

  .empty {
    color: var(--fg-4);
  }
</style>
