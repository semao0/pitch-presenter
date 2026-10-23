<!-- Keyboard reference. Any key or click closes it. -->
<script lang="ts">
  import type { MessageKey } from '../lib/i18n';
  import { t } from '../lib/settings.svelte';

  let { onclose }: { onclose: () => void } = $props();

  const groups: { title: MessageKey; rows: [string[], MessageKey][] }[] = [
    {
      title: 'help.slides',
      rows: [
        [['→', 'Space', 'PgDn'], 'help.next'],
        [['←', 'PgUp'], 'help.prev'],
        [['Home', 'End'], 'help.firstLast'],
        [['12', 'Enter'], 'help.goto'],
        [['G'], 'help.overview'],
        [['B'], 'help.blank'],
      ],
    },
    {
      title: 'help.textTime',
      rows: [
        [['↑', '↓'], 'help.scroll'],
        [['+', '−'], 'help.font'],
        [['T'], 'help.timer'],
        [['Shift', 'R'], 'help.reset'],
      ],
    },
    {
      title: 'help.windows',
      rows: [
        [['A'], 'help.audience'],
        [['F'], 'help.fullscreen'],
        [['?'], 'help.this'],
      ],
    },
  ];
</script>

<div class="backdrop" role="presentation" onclick={onclose}>
  <div class="panel" role="dialog" aria-label={t('presenter.help')}>
    {#each groups as group (group.title)}
      <section>
        <h3 class="label">{t(group.title)}</h3>
        {#each group.rows as [keys, action] (action)}
          <div class="row">
            <span class="keys">{#each keys as key (key)}<kbd>{key}</kbd>{/each}</span>
            <span class="action">{t(action)}</span>
          </div>
        {/each}
      </section>
    {/each}
    <p class="footnote">{t('help.footnote')}</p>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: grid;
    place-items: center;
    background: var(--backdrop);
    backdrop-filter: blur(6px);
    animation: fade-in 0.14s ease-out;
  }

  .panel {
    display: grid;
    grid-template-columns: repeat(3, auto);
    gap: 8px 44px;
    padding: 30px 34px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius-l);
    box-shadow: var(--shadow);
  }

  h3 {
    margin: 0 0 12px;
  }

  .row {
    display: grid;
    grid-template-columns: 120px auto;
    align-items: center;
    gap: 14px;
    padding: 5px 0;
    font-size: 14px;
  }

  .keys {
    display: flex;
    gap: 4px;
  }

  .action {
    color: var(--fg-2);
  }

  .footnote {
    grid-column: 1 / -1;
    margin: 14px 0 0;
    font-size: 13px;
    color: var(--fg-3);
  }

  @media (max-width: 1100px) {
    .panel {
      grid-template-columns: auto;
    }
  }
</style>
