<script setup lang="ts">
// Symmetric n×n matrix editor. Values are stored column-major to match
// nalgebra's native layout (see model/default-params.toml for details).
// Index into flat: row r, col c → flat[c * n + r].
import { computed } from "vue";
import { Hint, NumberInput } from "cfasim-ui/components";
import { getField } from "../config/uiConfig";
import { useParams } from "../composables/useParams";

const props = defineProps<{
  path: string;
  modelValue: number[];
}>();
const emit = defineEmits<(e: "update:modelValue", v: number[]) => void>();

const { params } = useParams();
const cfg = computed(() => getField(props.path));
const n = computed(() => params.population_fraction_labels.length);

// type=integer -> numberType=integer, percent=false
// type=float -> numberType=float, percent=false
// type=percent -> numberType=float, percent=true
const numberType = computed<"integer" | "float">(() =>
  cfg.value.type === "integer" ? "integer" : "float",
);
const percent = computed(() => cfg.value.type === "percent");

function idx(row: number, col: number): number {
  return col * n.value + row;
}

function get(row: number, col: number): number {
  return props.modelValue[idx(row, col)] ?? 0;
}

// Upper triangle writes both (r,c) and (c,r) to preserve symmetry.
function update(row: number, col: number, value: number) {
  const next = [...props.modelValue];
  next[idx(row, col)] = value;
  next[idx(col, row)] = value;
  emit("update:modelValue", next);
}
</script>

<template>
  <div class="matrix-input">
    <label class="matrix-input__label">
      {{ cfg.label }}
      <Hint v-if="cfg.tooltip" :text="cfg.tooltip" />
    </label>
    <table class="matrix-input__table">
      <thead>
        <tr>
          <th></th>
          <th v-for="(label, c) in params.population_fraction_labels" :key="c">
            {{ label }}
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(rowLabel, r) in params.population_fraction_labels" :key="r">
          <th scope="row">{{ rowLabel }}</th>
          <td v-for="(_col, c) in params.population_fraction_labels" :key="c">
            <!-- If below the diagonal, make this an inert, "mirror" value,
                 but with the same formatting -->
            <NumberInput
              :model-value="get(r, c)"
              @update:model-value="update(r, c, $event)"
              :min="cfg.min"
              :max="cfg.max"
              :step="cfg.step"
              hide-label
              :number-type="numberType"
              :percent="percent"
              :inert="c < r"
              :class="{ 'matrix-input__mirror': c < r }"
              live
            />
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.matrix-input {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}
.matrix-input__label {
  font-size: 0.875rem;
  opacity: 0.85;
}
.matrix-input__table {
  border-collapse: collapse;
  font-size: 0.875rem;
}
.matrix-input__table th,
.matrix-input__table td {
  padding: 0.25rem;
  text-align: left;
  vertical-align: middle;
}
.matrix-input__table thead th {
  font-weight: 500;
  opacity: 0.7;
}
.matrix-input__mirror {
  opacity: 0.4;
  font-variant-numeric: tabular-nums;
}
</style>
