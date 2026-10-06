<script setup lang="ts">
import { LineChart } from "cfasim-ui/charts";
import ChartTooltipContent from "./ChartTooltipContent.vue";
import {
  TICK_LABEL_STYLE,
  Y_LABEL_CHART_PADDING,
  type ChartData,
} from "../utils/chartScale";

defineProps<{
  data: ChartData;
  filename: string;
  height?: number;
  yLabel?: string;
}>();
</script>

<template>
  <LineChart
    :series="data.series"
    :x-labels="data.xLabels"
    :area-sections="data.areaSections"
    :y-label="yLabel"
    :chart-padding="yLabel ? Y_LABEL_CHART_PADDING : undefined"
    :tick-label-style="TICK_LABEL_STYLE"
    :filename="filename"
    :height="height ?? 200"
    :y-min="0"
    :x-min="0"
    tooltip-trigger="hover"
    tooltip-clamp="window"
  >
    <template #tooltip="{ index, values }">
      <ChartTooltipContent
        :index="index"
        :values="values"
        :x-labels="data.xLabels"
        :series="data.series"
        :raw-by-series="data.rawBySeries"
      />
    </template>
  </LineChart>
</template>
