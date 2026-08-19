<script lang="ts">
  import { apiBase, appName } from "$lib/config";
</script>

<svelte:head>
  <title>{appName}</title>
</svelte:head>

<section>
  <h1>{appName}</h1>
  <p>The Svelte application is running.</p>

  <!-- Shown rather than defaulted, so an unset PUBLIC_API_BASE is something
       the first person to open the page notices. -->
  {#if apiBase}
    <p>Talking to <code>{apiBase}</code></p>
  {:else}
    <p>No API base is configured. Set <code>PUBLIC_API_BASE</code>.</p>
  {/if}
</section>
