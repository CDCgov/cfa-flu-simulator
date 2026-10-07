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
import {
  pickScale,
  scale,
  TICK_LABEL_STYLE,
  Y_LABEL_CHART_PADDING,
} from "../utils/chartScale";

const props = defineProps<{
  results: ModelOutputExport | null;
}>();

const { params } = useParams();

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
  const tested = rows.map(
    (r) => r.grouped_values.reduce((a, b) => a + b, 0) * params.p_test_sympto,
  );
  const sc = pickScale(Math.max(...tested));
  const xLabels = rows.map((r) => String(Math.round(r.time)));
  const series: Series[] = [
    {
      data: scale(tested, sc.divisor),
      color: "var(--accent)",
      strokeWidth: 2,
    },
  ];
  const peakIdx = tested.indexOf(Math.max(...tested));
  return {
    series,
    xLabels,
    scale: sc,
    rawBySeries: [tested],
    peak: { value: tested[peakIdx], day: Math.round(rows[peakIdx].time) },
    total: tested.reduce((a, b) => a + b, 0),
  };
});

const approx = new Intl.NumberFormat("en-US", { maximumSignificantDigits: 2 });
// Non-breaking hyphen, so "200-day" never wraps after the dash.
const simulation = computed(() => `${params.days}\u2011day simulation`);

const testedDescription = computed(() => {
  const chart = testedChart.value;
  if (!chart) return "";
  return (
    `Tests given each day to newly symptomatic people, at ${fmtPct(params.p_test_sympto)} ` +
    `of symptomatic infections. Testing peaks at about ${approx.format(chart.peak.value)} ` +
    `on day ${chart.peak.day}, with about ${approx.format(chart.total)} tests over the ` +
    `whole ${simulation.value}.`
  );
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
  // First day the probability reaches each threshold; null if it never does.
  const milestones: { pct: number; day: number | null }[] = [];
  for (const threshold of [0.25, 0.75]) {
    const idx = trimmed.findIndex((p) => p.value >= threshold);
    const pct = threshold * 100;
    milestones.push({ pct, day: idx < 0 ? null : Math.round(trimmed[idx].time) });
    if (idx < 0) continue;
    annotations.push({
      x: idx,
      y: pct,
      text: `**≥${pct}%** Day ${Math.round(trimmed[idx].time)}`,
      offset: { x: 8, y: -6 },
      fontSize: 14,
      pointer: "ruleY",
      lineDash: "4 4",
      color: "var(--accent)",
    });
  }
  return { series, xLabels, annotations, milestones };
});

const pDetectDescription = computed(() => {
  const chart = pDetectChart.value;
  if (!chart) return "";
  const [low, high] = chart.milestones;
  const detected = "that public health has detected at least one case";
  if (low.day === null) {
    return `Under these settings the chance ${detected} stays below ${low.pct}% for the whole ${simulation.value}.`;
  }
  const rest =
    high.day === null
      ? `but the chance never reaches ${high.pct}% within the ${simulation.value}`
      : `and a ${high.pct}% chance by day ${high.day}`;
  return `Under these settings there is a ${low.pct}% chance ${detected} by day ${low.day}, ${rest}.`;
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
  <section class="results__section detection" id="detection">
    <h1>Probability of Detecting at Least One Case</h1>
    <p class="results__subtitle">{{ subtitle }}</p>

    <div class="detection__layout">
      <aside class="detection__controls">
        <h3>Detection</h3>
        <ParamField
          path="p_test_sympto"
          v-model="params.p_test_sympto"
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
      <div class="detection__charts">
        <div class="detection__chart">
          <h3>Symptomatic Cases Tested</h3>
          <p class="detection__chart-desc">{{ testedDescription }}</p>
          <LineChart
            v-if="testedChart"
            :series="testedChart.series"
            :x-labels="testedChart.xLabels"
            :y-label="`Cases Tested${testedChart.scale.unit ? ` (${testedChart.scale.unit})` : ''}`"
            :chart-padding="Y_LABEL_CHART_PADDING"
            :tick-label-style="TICK_LABEL_STYLE"
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
          <p class="detection__chart-desc">{{ pDetectDescription }}</p>
          <LineChart
            v-if="pDetectChart"
            :series="pDetectChart.series"
            :x-labels="pDetectChart.xLabels"
            :annotations="pDetectChart.annotations"
            y-label="Probability (%)"
            :chart-padding="Y_LABEL_CHART_PADDING"
            :tick-label-style="TICK_LABEL_STYLE"
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

    </div>
  </section>
</template>

<style scoped>
.detection {
  container-type: inline-size;
}
.detection__layout {
  margin-top: var(--space-6);
  display: grid;
  grid-template-columns: 320px minmax(0, 1fr);
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
.detection__chart-desc {
  margin: 0 0 var(--space-3);
  max-width: 70ch;
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}
.detection__controls {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: var(--space-6);
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
}
</style>
