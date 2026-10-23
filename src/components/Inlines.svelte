<!-- Renders inline note markup (bold, italic, highlight, code) recursively. -->
<script lang="ts">
  import type { Inline } from '../lib/types';
  import Self from './Inlines.svelte';

  let { nodes }: { nodes: Inline[] } = $props();
</script>

{#each nodes as node, i (i)}
  {#if node.kind === 'text'}{node.text}{:else if node.kind === 'code'}<code>{node.text}</code
    >{:else if node.kind === 'strong'}<strong><Self nodes={node.children} /></strong
    >{:else if node.kind === 'em'}<em><Self nodes={node.children} /></em
    >{:else}<mark><Self nodes={node.children} /></mark>{/if}
{/each}
