<script lang="ts">
  import "../app.css";
  import { invoke } from "@tauri-apps/api/core";
  import { resolve } from "$app/paths";
  import { onMount } from "svelte";
  import { themeService } from "$lib/services/theme-service.svelte";

  const settingsPath = resolve('/settings');

  let name = $state("");
  let greetMsg = $state("");

  async function greet(event: Event) {
    event.preventDefault();
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    greetMsg = await invoke("greet", { name });
  }

  onMount(async () => {
    await themeService.loadTheme();
  });

</script>

<main class="container">
  <h1>Welcome to Bible Nav</h1>

  <div class="row">
    <a href={settingsPath}>
      Application Settings
    </a>
  </div>

  <form class="row" onsubmit={greet}>
    <input id="greet-input" placeholder="Enter a name..." bind:value={name} />
    <button type="submit">Greet</button>
  </form>
  <p>{greetMsg}</p>
</main>

