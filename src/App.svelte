<!-- Main window: the start screen until a deck is open, then the presenter. -->
<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import { api, events } from './lib/api';
  import type { Deck } from './lib/types';
  import Presenter from './views/Presenter.svelte';
  import Start from './views/Start.svelte';

  let deck = $state.raw<Deck | null>(null);
  /** Until the backend says whether a deck is open, render nothing (no flash). */
  let ready = $state(false);

  onMount(() => {
    // After a page reload the session is still alive in the backend.
    void api.currentDeck().then((current) => {
      deck = current;
      ready = true;
    });
    return events.onDeckUpdated((updated) => {
      if (deck && updated.id === deck.id) deck = updated;
    });
  });

  function opened(next: Deck) {
    deck = next;
    void getCurrentWindow().maximize();
  }

  function closed() {
    deck = null;
    void getCurrentWindow().unmaximize();
  }

  $effect(() => {
    if (!deck) void getCurrentWindow().setTitle('Pitch Presenter');
  });
</script>

{#if ready}
  {#if deck}
    {#key deck.id}
      <Presenter {deck} onclose={closed} />
    {/key}
  {:else}
    <Start onopen={opened} />
  {/if}
{/if}
