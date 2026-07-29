<script lang="ts">
  import "../app.css";
  import { goto } from "$app/navigation";
  import { listen } from "@tauri-apps/api/event";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { syncTheme } from "$lib/theme";

  let { children } = $props();

  // Load settings immediately on initialization
  settingsStore.load();

  // The stored mode is the only input to the theme; everything else — resolving
  // "system", writing data-theme, following the OS — lives in $lib/theme.
  //
  // Returning syncTheme's teardown is what makes switching away from "system"
  // safe: Svelte runs the previous cleanup before re-running the effect, so the
  // OS colour scheme subscription is dropped and can no longer overwrite an
  // explicit light or dark choice.
  $effect(() => {
    const mode = settingsStore.settings?.theme;
    // Settings have not arrived (or failed to load) — leave whatever the
    // pre-paint script applied in place rather than flashing a guess.
    if (mode === undefined) return;
    return syncTheme(mode);
  });

  // The backend cannot change the route on its own, so tray items that need a
  // specific view (e.g. "Settings") emit this event instead. The listener lives
  // in the layout because it must work whichever page is currently mounted.
  //
  // Registered in an $effect so the cleanup actually runs — an async onMount
  // returns a Promise and Svelte then never calls the function it returns.
  $effect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    listen<string>("navigate", (event) => {
      if (event.payload) goto(event.payload);
    }).then((un) => {
      // The effect can be torn down before listen() resolves.
      if (disposed) un();
      else unlisten = un;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

{@render children()}
