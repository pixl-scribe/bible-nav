<script lang="ts">
    import { resolve } from "$app/paths";
    const homePath = resolve('/');

    let currentTheme = '';

    function setTheme(event: Event) {
        const select = event.target as HTMLSelectElement;
        const theme = select.value;
        document.documentElement.setAttribute('data-theme', theme);
        currentTheme = theme;
    }
</script>

<main class="container">
  <h1>Welcome to Bible Nav Configuration Page</h1>
  <div class="row">
    <a href={homePath}>
      Return to Home
    </a>
    <div>Current Theme: { currentTheme }</div>
    <div class="dropdown mb-72">
      <div tabindex="0" role="button" class="btn m-1">
        Theme
        <svg
            width="12px"
            height="12px"
            class="inline-block h-2 w-2 fill-current opacity-60"
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 2048 2048">
          <path d="M1799 349l242 241-1017 1017L7 590l242-241 775 775 775-775z"></path>
        </svg>
      </div>
      <ul tabindex="-1" class="dropdown-content bg-base-300 rounded-box z-1 w-52 p-2 shadow-2xl">
        <li>
          <input
              type="radio"
              name="theme-dropdown"
              class="theme-controller w-full btn btn-sm btn-block btn-ghost justify-start"
              aria-label="Default"
              value="default"
              onchange={setTheme} />
        </li>
        <li>
          <input
              type="radio"
              name="theme-dropdown"
              class="theme-controller w-full btn btn-sm btn-block btn-ghost justify-start"
              aria-label="Light"
              value="light"
              onchange={setTheme} />
        </li>
        <li>
          <input
              type="radio"
              name="theme-dropdown"
              class="theme-controller w-full btn btn-sm btn-block btn-ghost justify-start"
              aria-label="Dark"
              value="dark"
              onchange={setTheme} />
        </li>
        <li>
          <input
              type="radio"
              name="theme-dropdown"
              class="theme-controller w-full btn btn-sm btn-block btn-ghost justify-start"
              aria-label="Autumn"
              value="autumn"
              onchange={setTheme} />
        </li>
        <li>
          <input
              type="radio"
              name="theme-dropdown"
              class="theme-controller w-full btn btn-sm btn-block btn-ghost justify-start"
              aria-label="Cupcake"
              value="cupcake"
              onchange={setTheme} />
        </li>
        <li>
          <input
              type="radio"
              name="theme-dropdown"
              class="theme-controller w-full btn btn-sm btn-block btn-ghost justify-start"
              aria-label="Dim"
              value="dim"
              onchange={setTheme} />
        </li>
        <li>
          <input
              type="radio"
              name="theme-dropdown"
              class="theme-controller w-full btn btn-sm btn-block btn-ghost justify-start"
              aria-label="Emerald"
              value="emerald"
              onchange={setTheme} />
        </li>
        <li>
          <input
              type="radio"
              name="theme-dropdown"
              class="theme-controller w-full btn btn-sm btn-block btn-ghost justify-start"
              aria-label="Night"
              value="night"
              onchange={setTheme} />
        </li>
        <li>
          <input
              type="radio"
              name="theme-dropdown"
              class="theme-controller w-full btn btn-sm btn-block btn-ghost justify-start"
              aria-label="Nord"
              value="nord"
              onchange={setTheme} />
        </li>
        <li>
          <input
              type="radio"
              name="theme-dropdown"
              class="theme-controller w-full btn btn-sm btn-block btn-ghost justify-start"
              aria-label="Retro"
              value="retro"
              onchange={setTheme} />
        </li>
      </ul>
    </div>
  </div>
</main>

