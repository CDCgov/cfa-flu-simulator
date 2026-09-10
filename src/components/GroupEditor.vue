<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { NumberInput, Toggle } from "cfasim-ui/components";
import { getField } from "../config/uiConfig";
import { useParams } from "../composables/useParams";

const props = defineProps<{
  path: string;
  modelValue: number[];
}>();
const emit = defineEmits<(e: "update:modelValue", v: number[]) => void>();

const { params } = useParams();
const cfg = computed(() => getField(props.path));

// "All Ages" mode: all groups share the first group's value. We detect this
// initially by checking if all values are equal.
const allMode = ref(props.modelValue.every((v) => v === props.modelValue[0]));

function update(index: number, value: number) {
  if (allMode.value) {
    emit(
      "update:modelValue",
      props.modelValue.map(() => value),
    );
  } else {
    const next = [...props.modelValue];
    next[index] = value;
    emit("update:modelValue", next);
  }
}

// When "All Ages" is toggled on, snap other values to index 0.
watch(allMode, (on) => {
  if (on && props.modelValue.length > 0) {
    emit(
      "update:modelValue",
      props.modelValue.map(() => props.modelValue[0]),
    );
  }
});

const numberType = computed<"integer" | "float">(() =>
  cfg.value.type === "integer" ? "integer" : "float",
);
const percent = computed(() => cfg.value.type === "percent");
</script>

<template>
  <div class="group-editor">
    <div v-if="allMode" class="group-editor__all">
      <NumberInput
        :label="cfg.label"
        :hint="cfg.tooltip"
        :model-value="modelValue[0]"
        @update:model-value="update(0, $event)"
        :min="cfg.min"
        :max="cfg.max"
        :step="cfg.step"
        :slider="cfg.slider"
        :percent="percent"
        :number-type="numberType"
        live
      />
      <Toggle v-model="allMode" label="All Ages" class="group-editor__toggle" />
    </div>
    <template v-else>
      <div class="group-editor__header">
        <span class="group-editor__label">{{ cfg.label }}</span>
        <Toggle v-model="allMode" label="All Ages" />
      </div>
      <div class="group-editor__grid">
      <div
        v-for="(value, i) in modelValue"
        :key="i"
        class="group-editor__cell"
      >
        <NumberInput
          :label="params.population_fraction_labels[i]"
          :model-value="value"
          @update:model-value="update(i, $event)"
          :min="cfg.min"
          :max="cfg.max"
          :step="cfg.step"
          :percent="percent"
          :number-type="numberType"
          live
        />
      </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.group-editor {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}
.group-editor__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.group-editor__label {
  font-size: 0.875rem;
  opacity: 0.85;
}
.group-editor__grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
  gap: 0.5rem;
}
.group-editor__all {
  display: flex;
  align-items: flex-end;
  gap: 0.5rem;
}
.group-editor__all > :first-child {
  flex: 1;
}
</style>
