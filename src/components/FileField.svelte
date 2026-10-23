<!-- A file picker row: shows the chosen file's name and folder. -->
<script lang="ts">
  import { baseName, dirName } from '../lib/api';
  import { t } from '../lib/settings.svelte';
  import Icon, { type IconName } from './Icon.svelte';

  interface Props {
    label: string;
    path: string | null;
    placeholder: string;
    icon: IconName;
    hint?: string;
    disabled?: boolean;
    onpick: () => void;
    onclear?: () => void;
  }

  let { label, path, placeholder, icon, hint, disabled = false, onpick, onclear }: Props = $props();
</script>

<div class="field">
  <span class="label">{label}</span>
  <div class="row">
    <button class="file" class:empty={!path} {disabled} onclick={onpick} title={path ?? placeholder}>
      <span class="icon"><Icon name={icon} size={20} /></span>
      <span class="text">
        <span class="name">{path ? baseName(path) : placeholder}</span>
        {#if path}
          <span class="dir path"><bdo dir="ltr">{dirName(path)}</bdo></span>
        {:else if hint}
          <span class="dir">{hint}</span>
        {/if}
      </span>
    </button>
    {#if path && onclear}
      <button class="icon-btn" {disabled} onclick={onclear} title={t('start.remove')} aria-label={t('start.remove')}>
        <Icon name="close" size={16} />
      </button>
    {/if}
  </div>
</div>

<style>
  .field {
    display: grid;
    gap: 8px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .file {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 14px;
    text-align: left;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius);
    transition: border-color 0.15s;
  }

  .file:hover:not(:disabled) {
    border-color: var(--fg-3);
  }

  .file:disabled {
    cursor: default;
  }

  .file.empty {
    border-style: dashed;
  }

  .icon {
    color: var(--fg);
  }

  .empty .icon {
    color: var(--fg-4);
  }

  .text {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  .name,
  .dir {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .name {
    font-weight: 550;
  }

  .empty .name {
    color: var(--fg-2);
    font-weight: 400;
  }

  .dir {
    font-size: 12px;
    color: var(--fg-3);
  }

  .path {
    /* Truncate long paths from the start: the nearest folders matter most. */
    direction: rtl;
    text-align: left;
  }
</style>
