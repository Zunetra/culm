<script setup lang="ts">
import type { NuxtError } from "#app";

// Nuxt renders this instead of the whole app when a fatal error escapes, which
// is also why it cannot use a layout: the layout is part of what may have
// failed.
const props = defineProps<{ error: NuxtError }>();

// The message is only safe to show for the errors the application raised
// deliberately. An unexpected 500 carries whatever the stack happened to say.
const message: string =
  props.error.statusCode === 404
    ? "That page does not exist."
    : "Something went wrong.";

function reset(): void {
  clearError({ redirect: "/" });
}
</script>

<template>
  <section class="error">
    <h1>{{ props.error.statusCode }}</h1>
    <p>{{ message }}</p>
    <button type="button" @click="reset">Back to the start</button>
  </section>
</template>

<style scoped>
.error {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
  padding: 4rem 1.5rem;
  font-family: system-ui, sans-serif;
  text-align: center;
}
</style>
