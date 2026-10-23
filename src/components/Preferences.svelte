<!-- Theme and language switches. -->
<script lang="ts">
  import { settings, t } from '../lib/settings.svelte';
  import Icon from './Icon.svelte';

  let { language = true }: { language?: boolean } = $props();
</script>

<div class="preferences">
  {#if language}
    <button class="icon-btn lang" onclick={() => settings.toggleLang()} title={t('settings.lang')}>
      {settings.lang === 'ru' ? 'EN' : 'RU'}
    </button>
  {/if}
  <button
    class="icon-btn"
    onclick={() => settings.toggleTheme()}
    title={t(settings.theme === 'dark' ? 'settings.theme.light' : 'settings.theme.dark')}
  >
    <Icon name={settings.theme === 'dark' ? 'sun' : 'moon'} />
  </button>
</div>

<style>
  .preferences {
    display: flex;
    gap: 2px;
  }

  .lang {
    padding: 0 8px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
  }
</style>
