import '@fontsource-variable/inter';
import './app.css';

import { getCurrentWindow } from '@tauri-apps/api/window';
import { error as logError } from '@tauri-apps/plugin-log';
import { mount } from 'svelte';
import { AUDIENCE_WINDOW } from './lib/api';
import { settings } from './lib/settings.svelte';
import App from './App.svelte';
import Audience from './views/Audience.svelte';

// Both windows load the same bundle; the window label picks the view.
const isAudience = getCurrentWindow().label === AUDIENCE_WINDOW;

// No browser context menu ("Reload", "Inspect") in a desktop app; views that
// use right-click handle it themselves.
if (!import.meta.env.DEV) {
  document.addEventListener('contextmenu', (e) => e.preventDefault());
}

// Uncaught frontend errors go to the app log (next to the backend's), so bug
// reports have the full picture.
window.addEventListener('error', (e) => void logError(`${e.message} at ${e.filename}:${e.lineno}`));
window.addEventListener('unhandledrejection', (e) => void logError(`unhandled rejection: ${String(e.reason)}`));

// Set before the first paint so a dark-theme launch doesn't flash light.
document.documentElement.dataset.theme = settings.theme;

const target = document.getElementById('app')!;
export default mount(isAudience ? Audience : App, { target });
