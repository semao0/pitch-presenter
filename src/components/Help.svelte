<!-- Keyboard reference. Any key or click closes it. -->
<script lang="ts">
  let { onclose }: { onclose: () => void } = $props();

  const groups: { title: string; rows: [string[], string][] }[] = [
    {
      title: 'Слайды',
      rows: [
        [['→', 'Space', 'PgDn'], 'вперёд · клик по слайду'],
        [['←', 'PgUp'], 'назад · правый клик'],
        [['Home', 'End'], 'первый · последний'],
        [['12', 'Enter'], 'перейти к слайду'],
        [['G'], 'все слайды'],
        [['B'], 'скрыть экран'],
      ],
    },
    {
      title: 'Текст и время',
      rows: [
        [['↑', '↓'], 'прокрутить текст'],
        [['+', '−'], 'размер текста'],
        [['T'], 'таймер: старт · пауза'],
        [['Shift', 'R'], 'сбросить таймер'],
      ],
    },
    {
      title: 'Окна',
      rows: [
        [['A'], 'открыть окно показа'],
        [['F'], 'полный экран (в обоих окнах)'],
        [['?'], 'эта справка'],
      ],
    },
  ];
</script>

<div class="backdrop" role="presentation" onclick={onclose}>
  <div class="panel" role="dialog" aria-label="Клавиши">
    {#each groups as group (group.title)}
      <section>
        <h3>{group.title}</h3>
        {#each group.rows as [keys, action] (action)}
          <div class="row">
            <span class="keys">{#each keys as key (key)}<kbd>{key}</kbd>{/each}</span>
            <span class="action">{action}</span>
          </div>
        {/each}
      </section>
    {/each}
    <p class="footnote">Клавиши работают и когда в фокусе окно показа.</p>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: grid;
    place-items: center;
    background: rgb(6 7 9 / 70%);
    backdrop-filter: blur(4px);
    animation: fade 0.14s ease-out;
  }

  .panel {
    display: grid;
    grid-template-columns: repeat(3, auto);
    gap: 8px 40px;
    padding: 28px 32px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius-l);
    box-shadow: var(--shadow);
  }

  h3 {
    margin: 0 0 12px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-4);
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

  @keyframes fade {
    from {
      opacity: 0;
    }
  }

  @media (max-width: 1100px) {
    .panel {
      grid-template-columns: auto;
    }
  }
</style>
