<script setup lang="ts">
import { Toggle } from "cfasim-ui/components";

defineProps<{
  label: string;
  enabled: boolean;
}>();
defineEmits<(e: "update:enabled", v: boolean) => void>();
</script>

<template>
  <section class="mitigation-section" :data-enabled="enabled">
    <header class="mitigation-section__header">
      <span class="mitigation-section__title">{{ label }}</span>
      <Toggle
        :label="enabled ? 'Enabled' : 'Disabled'"
        :model-value="enabled"
        @update:model-value="$emit('update:enabled', $event)"
      />
    </header>
    <div v-if="enabled" class="mitigation-section__body">
      <slot />
    </div>
  </section>
</template>

<style scoped>
.mitigation-section {
  margin: 0;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: var(--space-3);
  background: var(--color-bg-0);
}
.mitigation-section[data-enabled="true"] {
  border-color: color-mix(in srgb, var(--color-primary) 45%, transparent);
  background: color-mix(in srgb, var(--color-primary) 4%, var(--color-bg-0));
}
.mitigation-section__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
}
.mitigation-section__title {
  font-weight: 600;
  font-size: var(--font-size-sm);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}
.mitigation-section__body {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding-top: var(--space-3);
  margin-top: var(--space-3);
  border-top: 1px solid var(--color-border);
}
</style>
