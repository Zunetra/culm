<script setup lang="ts">
const config = useRuntimeConfig();
const appName: string = config.public.appName;
const apiBase: string = config.public.apiBase;

useHead({ title: appName });
</script>

<template>
  <section>
    <h1>{{ appName }}</h1>
    <p>The Nuxt application is running.</p>

    <!-- Shown rather than defaulted, so an unset NUXT_PUBLIC_API_BASE is
         something the first person to open the page notices. -->
    <p v-if="apiBase">Talking to <code>{{ apiBase }}</code></p>
    <p v-else>No API base is configured. Set <code>NUXT_PUBLIC_API_BASE</code>.</p>
  </section>
</template>
