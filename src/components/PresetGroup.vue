<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Expander, SelectBox } from "cfasim-ui/components";
import { useParams } from "../composables/useParams";
import { findMatchingPreset, type Preset } from "../config/presets";

// A preset dropdown over the fields it controls, which go in the default
// slot and render under a collapsible "Advanced settings" header.
const props = defineProps<{
  presets: Preset[];
  hint: string;
}>();

const { params } = useParams();

const options = computed(() =>
  props.presets.map((preset) => ({ value: preset.id, label: preset.label })),
);

// Derived from the parameters, not stored: a shared link or an import that
// matches a preset shows it, and editing any of its fields falls back to the
// "Custom" placeholder.
const matching = computed(() => findMatchingPreset(props.presets, params));
const selected = computed({
  get: () => matching.value?.id ?? "",
  set: (id: string | undefined) => {
    const preset = props.presets.find((candidate) => candidate.id === id);
    if (preset) Object.assign(params, structuredClone(preset.params));
  },
});

// Custom values are only visible in the advanced fields, so those start
// open, and reopen when a reset or import lands on Custom. Picking a preset
// leaves the section as the user had it.
const advancedOpen = ref(!matching.value);
watch(matching, (preset) => {
  if (!preset) advancedOpen.value = true;
});
</script>

<template>
  <div class="preset-group">
    <SelectBox
      v-model="selected"
      label="Preset"
      :hint="hint"
      placeholder="Custom"
      :options="options"
    />
    <Expander v-model:open="advancedOpen" label="Advanced settings">
      <slot />
    </Expander>
  </div>
</template>

<style scoped>
.preset-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

/* The library renders expander labels as tracked-out caps. */
.preset-group :deep(.expander-trigger) {
  text-transform: none;
  letter-spacing: normal;
}
</style>
