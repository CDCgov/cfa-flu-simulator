<script setup lang="ts">
import { computed } from "vue";
import { NumberInput } from "cfasim-ui/components";
import { getField } from "../config/uiConfig";

const props = defineProps<{
  path: string;
  modelValue: number;
  // Render this field and its `range_with` companion as one two-handle
  // slider: `modelValue` is the lower handle, `upperValue` the upper.
  paired?: boolean;
  upperValue?: number;
  // Optional runtime overrides (e.g. community.start max depends on `days`)
  min?: number;
  max?: number;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", v: number): void;
  (e: "update:upperValue", v: number): void;
}>();

const cfg = computed(() => getField(props.path));

const numberType = computed<"integer" | "float">(() =>
  cfg.value.type === "integer" ? "integer" : "float",
);
const percent = computed(() => cfg.value.type === "percent");

// uiConfig validates the pairing at load, so `range_with` is trustworthy here.
const paired = computed(() => !!props.paired && !!cfg.value.range_with);
const label = computed(() =>
  paired.value ? (cfg.value.range_label ?? cfg.value.label) : cfg.value.label,
);
const hint = computed(() =>
  paired.value
    ? (cfg.value.range_tooltip ?? cfg.value.tooltip)
    : cfg.value.tooltip,
);
</script>

<template>
  <!-- Separate elements: NumberInput infers range mode from the bindings,
       and warns if the default v-model is bound alongside lower/upper.
       `bar="segments"` bands 0->dose 1 and dose 1->dose 2, so the track reads
       as two point estimates rather than one interval. -->
  <NumberInput
    v-if="paired"
    :lower="modelValue"
    @update:lower="emit('update:modelValue', $event)"
    :upper="upperValue"
    @update:upper="emit('update:upperValue', $event)"
    bar="segments"
    :label="label"
    :hint="hint"
    :min="min ?? cfg.min"
    :max="max ?? cfg.max"
    :step="cfg.step"
    :percent="percent"
    :number-type="numberType"
    live
  />
  <NumberInput
    v-else
    :model-value="modelValue"
    @update:model-value="emit('update:modelValue', $event)"
    :label="label"
    :hint="hint"
    :min="min ?? cfg.min"
    :max="max ?? cfg.max"
    :step="cfg.step"
    :slider="cfg.slider"
    :percent="percent"
    :number-type="numberType"
    live
  />
</template>
