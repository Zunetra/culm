<script setup lang="ts">
import { onErrorCaptured, ref } from "vue";

import { apiBase, appName } from "./config";

// Without a meta framework nothing catches a render error, so the boundary is
// part of what the layer ships rather than something to add later.
const failed = ref<boolean>(false);

onErrorCaptured((error): boolean => {
  console.error("unhandled error", error);
  failed.value = true;
  // Stopping propagation here is what keeps the whole app from unmounting.
  return false;
});
</script>

<template>
  <section v-if="failed">
    <h1>Something went wrong</h1>
    <p>The page failed to load.</p>
    <button type="button" @click="failed = false">Try again</button>
  </section>

  <section v-else>
    <h1>{{ appName }}</h1>
    <p>The Vue application is running.</p>

    <!-- Shown rather than defaulted, so an unset VITE_API_BASE is something
         the first person to open the page notices. -->
    <p v-if="apiBase">
      Talking to <code>{{ apiBase }}</code>
    </p>
    <p v-else>No API base is configured. Set <code>VITE_API_BASE</code>.</p>
  </section>
</template>
