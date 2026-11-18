<script setup lang="ts">
import type { Series } from "cfasim-ui/charts";

defineProps<{
  index: number;
  values: { value: number; color: string; seriesIndex: number }[];
  xLabels: string[];
  series: Series[];
}>();

function formatScaled(v: number): string {
  if (v >= 1e6) {
    const n = v / 1e6;
    const d = n >= 100 ? 0 : n >= 10 ? 1 : 2;
    return `${n.toFixed(d)} million`;
  }
  if (v >= 1e3) {
    const n = v / 1e3;
    const d = n >= 100 ? 0 : n >= 10 ? 1 : 2;
    return `${n.toFixed(d)} thousand`;
  }
  return Math.round(v).toLocaleString();
}
</script>

<template>
  <div class="tt">
    <div class="tt__day">Day {{ xLabels[index] }}</div>
    <div v-for="(v, i) in values" :key="i" class="tt__row">
      <template v-if="series[v.seriesIndex].legend">
        <span class="tt__swatch" :style="{ background: v.color }" />
        <span class="tt__label">{{ series[v.seriesIndex].legend }}</span>
      </template>
      <span class="tt__value">{{ formatScaled(v.value) }}</span>
    </div>
  </div>
</template>

<style scoped>
/* LineChart provides the .chart-tooltip-content wrapper (card + padding +
   shadow). This is just layout inside it. */
.tt {
  font-size: 0.8rem;
}
.tt__day {
  font-weight: 600;
  margin-bottom: 0.25rem;
  opacity: 0.7;
}
.tt__row {
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: 0.5rem;
  align-items: center;
}
.tt__swatch {
  display: inline-block;
  width: 0.6rem;
  height: 0.6rem;
  border-radius: 2px;
}
.tt__label,
.tt__value {
  white-space: nowrap;
}
.tt__value {
  font-variant-numeric: tabular-nums;
}
</style>
