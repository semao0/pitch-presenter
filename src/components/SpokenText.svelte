<script lang="ts">
  let { text, offset, position }: { text: string; offset: number; position: number } = $props();
  const parts = $derived.by(() => {
    let index = offset;
    return (text.match(/[\p{L}\p{N}]+|[^\p{L}\p{N}]+/gu) ?? []).map((part) => ({
      text: part,
      index: /[\p{L}\p{N}]/u.test(part) ? index++ : -1,
    }));
  });
</script>

{#each parts as part, index (index)}{#if part.index < 0}{part.text}{:else}<span
  class:spoken={part.index >= 0 && part.index < position}
  class:current={part.index >= 0 && part.index === position - 1}
  data-speech-current={part.index >= 0 && part.index === position - 1 ? '' : undefined}
>{part.text}</span>{/if}{/each}

<style>
  .spoken { opacity: 0.48; }
  .current { opacity: 1; text-decoration: underline; text-decoration-color: var(--accent); text-underline-offset: 0.2em; }
</style>
