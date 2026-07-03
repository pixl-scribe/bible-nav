<script lang="ts">
  import "../app.css";
  import { invoke } from "@tauri-apps/api/core";
  import { resolve } from "$app/paths";
  import { onMount } from "svelte";
  import { themeService } from "$lib/services/theme-service.svelte";
  import Database from '@tauri-apps/plugin-sql';

  const settingsPath = resolve('/settings');

  let name = $state("");
  let greetMsg = $state("");
  let db: Database;

  async function greet(event: Event) {
    event.preventDefault();
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    greetMsg = await invoke("greet", { name });
  }

  async function connect() {
    await db.execute(
      'INSERT INTO modules (name, description) VALUES ($1, $2);',
      ['kjv', 'king james version module']);
    const res = await db.select('SELECT * FROM modules;');
    console.log({ res });
  }

  onMount(async () => {
    await themeService.loadTheme();
    db = await Database.load('sqlite:biblenav.db');
  });

</script>

<main class="container">
  <h1>Welcome to Bible Nav</h1>

  <div class="row">
    <a href={settingsPath}>
      Application Settings
    </a>
  </div>

  <div>
    <button class="btn btn-primary" onclick={connect}>Connect to DB</button>
  </div>
  <form class="row" onsubmit={greet}>
    <input id="greet-input" placeholder="Enter a name..." bind:value={name} />
    <button class="btn btn-primary" type="submit">Greet</button>
  </form>
  <p>{greetMsg}</p>
</main>

