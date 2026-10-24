<!-- Keyboard reference. Any key or click closes it. -->
<script lang="ts">
  import type { MessageKey } from '../lib/i18n';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { t } from '../lib/settings.svelte';

  let { onclose }: { onclose: () => void } = $props();

  const credits = $derived<[string, string][]>([
    ['GigaAM-v3 (SberDevices, MIT)', 'https://huggingface.co/ai-sage/GigaAM-v3'],
    ['transcribe.cpp', 'https://github.com/handy-computer/transcribe.cpp'],
    ['Silero VAD', 'https://github.com/snakers4/silero-vad'],
    [t('help.licenses'), 'https://github.com/semao0/pitch-presenter/blob/main/THIRD_PARTY_NOTICES.md'],
  ]);

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
        [['L'], 'help.auto'],
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
    <!-- Model attribution, as its MIT license asks. Links open in the browser. -->
    <p class="footnote credits">
      {t('help.speechBy')}
      {#each credits as [label, url], i (url)}{#if i > 0}{' · '}{/if}<a href={url} onclick={(event) => {
        event.preventDefault();
        event.stopPropagation();
        void openUrl(url);
      }}>{label}</a>{/each}
    </p>
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

  .credits a {
    color: var(--accent);
    text-decoration: none;
  }

  .credits a:hover {
    text-decoration: underline;
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
