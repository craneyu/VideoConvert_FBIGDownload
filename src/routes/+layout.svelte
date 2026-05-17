<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";

  let { children } = $props();

  onMount(() => {
    settingsStore.load();
  });
</script>

{@render children()}
