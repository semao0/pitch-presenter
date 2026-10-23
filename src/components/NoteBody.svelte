<!-- Speaker note text: paragraphs, bullet lists and cues (remarks to self). -->
<script lang="ts">
  import type { Block } from '../lib/types';
  import Inlines from './Inlines.svelte';

  let { blocks }: { blocks: Block[] } = $props();
</script>

<div class="note">
  {#each blocks as block, i (i)}
    {#if block.kind === 'paragraph'}
      <p><Inlines nodes={block.content} /></p>
    {:else if block.kind === 'list'}
      <ul>
        {#each block.content as item, j (j)}
          <li><Inlines nodes={item} /></li>
        {/each}
      </ul>
    {:else}
      <p class="cue"><Inlines nodes={block.content} /></p>
    {/if}
  {:else}
    <p class="empty">Без текста</p>
  {/each}
</div>

<style>
  .note {
    font-size: var(--note-size, 30px);
    line-height: 1.55;
    max-width: 44ch;
    text-wrap: pretty;
  }

  p,
  ul {
    margin: 0 0 0.75em;
  }

  ul {
    padding-left: 1.1em;
  }

  li::marker {
    color: var(--fg-3);
  }

  .note :global(strong) {
    color: #fff;
    font-weight: 650;
  }

  .note :global(mark) {
    background: none;
    color: var(--warn);
  }

  .note :global(code) {
    font: 0.85em var(--mono);
    background: var(--surface-2);
    padding: 0 0.3em;
    border-radius: 4px;
  }

  .cue {
    font-size: 0.62em;
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
