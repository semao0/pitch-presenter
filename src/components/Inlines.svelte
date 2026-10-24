<!-- Renders inline note markup (bold, italic, highlight, code) recursively. -->
<script lang="ts">
  import type { Inline } from '../lib/types';
  import Self from './Inlines.svelte';
  import SpokenText from './SpokenText.svelte';
  import { inlineOffsets } from '../lib/speech';

  let { nodes, offset = 0, position = -1 }: { nodes: Inline[]; offset?: number; position?: number } = $props();
  const offsets = $derived(inlineOffsets(nodes, offset));
</script>

{#each nodes as node, i (i)}
  {#if node.kind === 'text'}{#if position < 0}{node.text}{:else}<SpokenText text={node.text} offset={offsets[i]!} {position} />{/if}{:else if node.kind === 'code'}<code>{#if position < 0}{node.text}{:else}<SpokenText text={node.text} offset={offsets[i]!} {position} />{/if}</code
    >{:else if node.kind === 'strong'}<strong><Self nodes={node.children} offset={offsets[i]!} {position} /></strong
    >{:else if node.kind === 'em'}<em><Self nodes={node.children} offset={offsets[i]!} {position} /></em
    >{:else}<mark><Self nodes={node.children} offset={offsets[i]!} {position} /></mark>{/if}
{/each}
