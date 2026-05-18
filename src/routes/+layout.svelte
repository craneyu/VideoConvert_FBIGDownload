<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";

  let { children } = $props();

  // Load settings immediately on initialization
  settingsStore.load();

  onMount(() => {
    // Initial load already handled, but keeping onMount for potential other logic
  });
</script>

{@render children()}
