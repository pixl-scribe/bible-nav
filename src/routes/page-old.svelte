<script lang="ts">
  import '../app.css';
  import { invoke } from '@tauri-apps/api/core';
  import { resolve } from '$app/paths';
  import { onMount } from 'svelte';
  import { themeService } from '$lib/services/theme-service.svelte.js';
  import Database from '@tauri-apps/plugin-sql';
  import { _ } from 'svelte-i18n';
  import LeftNav from '$lib/components/left-nav.svelte';

  const settingsPath = resolve('/settings');

  let name = $state('');
  let greetMsg = $state('');
  let db: Database;

  // eslint-disable-next-line no-undef
  async function greet(event: Event) {
    event.preventDefault();
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    greetMsg = await invoke('greet', { name });
  }

  async function connect() {
    await db.execute(
      'INSERT INTO modules (name, description) VALUES ($1, $2);',
      ['kjv', 'king james version module']
    );
    const res = await db.select('SELECT * FROM modules;');
    // eslint-disable-next-line no-undef
    console.log({ res });
  }

  onMount(async () => {
    await themeService.loadTheme();
    db = await Database.load('sqlite:biblenav.db');
  });
</script>

<main class="flex p-1">
  <div class="flex mr-1">
    <LeftNav />
  </div>
  <div class="flex">
    <h1>{$_('common.app-name')}</h1>
    <a href={settingsPath}>
      {$_('home.app-settings')}
    </a>
    <button class="btn btn-primary" onclick={connect}>Connect to DB</button>
    <form class="row" onsubmit={greet}>
      <input id="greet-input" placeholder="Enter a name..." bind:value={name} />
      <button class="btn btn-primary" type="submit">Greet</button>
    </form>
    <p>{greetMsg}</p>
  </div>
</main>
