<script setup lang="ts">
const config = useRuntimeConfig();
const appName: string = config.public.appName;
</script>

<template>
  <div class="shell">
    <header class="shell-header">
      <NuxtLink to="/">{{ appName }}</NuxtLink>
    </header>

    <main class="shell-main">
      <slot />
    </main>
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  font-family: system-ui, sans-serif;
}

.shell-header {
  padding: 1rem 1.5rem;
  border-bottom: 1px solid #e5e5e5;
  font-weight: 600;
}

.shell-main {
  flex: 1;
  padding: 1.5rem;
}
</style>
