<script lang="ts">
  import "../app.css";
  import { goto } from "$app/navigation";
  import { listen } from "@tauri-apps/api/event";
  import { settingsStore } from "$lib/stores/settings.svelte";

  let { children } = $props();

  // Load settings immediately on initialization
  settingsStore.load();

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
