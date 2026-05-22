<script setup lang="ts">
import { computed } from "vue";
import { LineChart, type Series, type ChartAnnotation } from "cfasim-ui/charts";
import ChartTooltipContent from "../components/ChartTooltipContent.vue";
import ParamField from "../components/ParamField.vue";
import {
  useParams,
  type ModelOutputExport,
  type OutputItemGrouped,
} from "../composables/useParams";
import { pickScale, scale } from "../utils/chartScale";

const props = defineProps<{
  results: ModelOutputExport | null;
}>();

const { params } = useParams();

const pTestSymptoPct = computed({
  get: () => params.p_test_sympto * 100,
  set: (v: number) => {
    params.p_test_sympto = v / 100;
  },
});

const UNMITIGATED_COLOR = "#9ca3af";

const symptomaticRows = computed<OutputItemGrouped[] | null>(() => {
  const r = props.results;
  if (!r) return null;
  return (
    r.output.Mitigated?.SymptomaticIncidence ??
    r.output.Unmitigated?.SymptomaticIncidence ??
    null
  );
});

const testedChart = computed(() => {
  const rows = symptomaticRows.value;
  if (!rows) return null;
  const allCases = rows.map((r) =>
    r.grouped_values.reduce((a, b) => a + b, 0),
  );
  const tested = allCases.map((v) => v * params.p_test_sympto);
  const sc = pickScale(Math.max(...allCases));
  const xLabels = rows.map((r) => String(Math.round(r.time)));
  const series: Series[] = [
    {
      data: scale(allCases, sc.divisor),
      color: UNMITIGATED_COLOR,
      legend: "All",
    },
    {
      data: scale(tested, sc.divisor),
      color: "var(--accent)",
      strokeWidth: 2,
      legend: "Tested",
    },
  ];
  return {
    series,
    xLabels,
    scale: sc,
    rawBySeries: [allCases, tested],
  };
});

const pDetectChart = computed(() => {
  const r = props.results;
  if (!r) return null;
  const pd = r.p_detect.Mitigated ?? r.p_detect.Unmitigated;
  if (!pd || pd.length === 0) return null;
  // Truncate at 5 steps after probability first hits 100%, or end of sim.
  const hitIdx = pd.findIndex((p) => p.value >= 1);
  const endIdx = hitIdx >= 0 ? Math.min(pd.length, hitIdx + 6) : pd.length;
  const trimmed = pd.slice(0, endIdx);
  const xLabels = trimmed.map((p) => String(Math.round(p.time)));
  const series: Series[] = [
    {
      data: trimmed.map((p) => p.value * 100),
      color: "var(--accent)",
      strokeWidth: 2,
      legend: "P(detect ≥ 1)",
    },
  ];
  const annotations: ChartAnnotation[] = [];
  for (const { threshold, label } of [
    { threshold: 0.25, label: "≥25%" },
    { threshold: 0.75, label: "≥75%" },
  ]) {
    const idx = trimmed.findIndex((p) => p.value >= threshold);
    if (idx < 0) continue;
    annotations.push({
      x: idx,
      y: threshold * 100,
      text: `**${label}** Day ${Math.round(trimmed[idx].time)}`,
      offset: { x: 8, y: -8 },
      pointer: "ruleY",
      lineDash: "4 4",
      color: "var(--accent)",
    });
  }
  return { series, xLabels, annotations };
});

function fmtPct(v: number, digits = 1): string {
  return `${(v * 100).toFixed(digits)}%`;
}

const subtitle = computed(
  () =>
    `Given ${fmtPct(params.p_test_sympto)} of new symptomatic infections tested, ` +
    `${fmtPct(params.p_test_forward, 0)} of tests forwarded to public health, ` +
    `and ${fmtPct(params.test_sensitivity, 0)} test sensitivity.`,
);
</script>

<template>
  <section class="results__section detection" data-otp-id="detection" id="detection">
    <h1>Probability of Detecting at Least One Case</h1>
    <p class="results__subtitle">{{ subtitle }}</p>

    <div class="detection__layout">
      <div class="detection__charts">
        <div class="detection__chart">
          <h3>Symptomatic Cases Tested</h3>
          <LineChart
            v-if="testedChart"
            :series="testedChart.series"
            :x-labels="testedChart.xLabels"
            :y-label="`Cases${testedChart.scale.unit ? ` (${testedChart.scale.unit})` : ''}`"
            filename="symptomatic-cases-tested"
            :height="180"
            :y-min="0"
            :x-min="0"
            tooltip-trigger="hover"
            tooltip-clamp="window"
          >
            <template #tooltip="{ index, values }">
              <ChartTooltipContent
                :index="index"
                :values="values"
                :x-labels="testedChart.xLabels"
                :series="testedChart.series"
                :raw-by-series="testedChart.rawBySeries"
              />
            </template>
          </LineChart>
        </div>

        <div class="detection__chart">
          <h3>Cumulative Probability of Detection</h3>
          <LineChart
            v-if="pDetectChart"
            :series="pDetectChart.series"
            :x-labels="pDetectChart.xLabels"
            :annotations="pDetectChart.annotations"
            y-label="Probability (%)"
            filename="cumulative-probability-of-detection"
            :height="220"
            :y-min="0"
            :x-min="0"
            tooltip-trigger="hover"
            tooltip-clamp="window"
          >
            <template #tooltip="{ index, values }">
              <ChartTooltipContent
                :index="index"
                :values="values"
                :x-labels="pDetectChart.xLabels"
                :series="pDetectChart.series"
                unit="%"
              />
            </template>
          </LineChart>
        </div>
      </div>

      <aside class="detection__controls">
        <h3>Detection</h3>
        <ParamField
          path="p_test_sympto"
          v-model="pTestSymptoPct"
        />
        <ParamField
          path="test_sensitivity"
          v-model="params.test_sensitivity"
        />
        <ParamField
          path="p_test_forward"
          v-model="params.p_test_forward"
        />
      </aside>
    </div>
  </section>
</template>

<style scoped>
.detection {
  container-type: inline-size;
}
.detection__layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 240px;
  gap: 1.5rem;
  align-items: start;
}
.detection__charts {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  min-width: 0;
}
.detection__chart h3 {
  margin: 0 0 0.25rem;
}
.detection__controls {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 0.75rem;
  border: 1px solid rgba(128, 128, 128, 0.2);
  border-radius: 4px;
}
.detection__controls h3 {
  margin: 0 0 0.25rem;
  font-size: 0.875rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  opacity: 0.7;
}
@container (max-width: 700px) {
  .detection__layout {
    grid-template-columns: 1fr;
  }
  .detection__controls {
    order: -1;
  }
}
</style>
