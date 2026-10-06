<script lang="ts">
export type ResultsTab = "charts" | "summary" | "detection";
</script>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import type { Series, AreaSection } from "cfasim-ui/charts";
import ChartPanel from "../components/ChartPanel.vue";
import SummaryView from "./SummaryView.vue";
import DetectionSection from "../sections/DetectionSection.vue";
import { useParams, type ModelOutputExport, type OutputItemGrouped, type OutputTypeLabel } from "../composables/useParams";
import { useModelRun } from "../composables/useModelRun";
import { pickScale, scale, type ChartData } from "../utils/chartScale";

const { params } = useParams();
const { results, running, error } = useModelRun();

const props = defineProps<{ tab: ResultsTab }>();

// The tabs share one scroll container, so start each panel from the top.
const root = ref<HTMLElement | null>(null);
watch(
  () => props.tab,
  async () => {
    await nextTick();
    root.value?.scrollIntoView({ block: "start" });
  },
);

// --- series extraction helpers ---------------------------------------------

function sumSeries(rows: OutputItemGrouped[]): number[] {
  return rows.map((r) => r.grouped_values.reduce((a, b) => a + b, 0));
}

function groupSeries(rows: OutputItemGrouped[], groupIndex: number): number[] {
  return rows.map((r) => r.grouped_values[groupIndex] ?? 0);
}

function xLabels(rows: OutputItemGrouped[]): string[] {
  return rows.map((r) => String(Math.round(r.time)));
}

// --- chart building --------------------------------------------------------

const UNMITIGATED_COLOR = "#9ca3af"; // neutral gray for the counterfactual

function buildChart(
  kind: OutputTypeLabel,
  extract: (rows: OutputItemGrouped[]) => number[],
  showAreaLegend = false,
): ChartData | null {
  const r: ModelOutputExport | null = results.value;
  if (!r) return null;

  const mitigated = r.output.Mitigated?.[kind];
  const unmitigated = r.output.Unmitigated?.[kind];
  const hasBoth = !!mitigated && !!unmitigated;
  const primary = mitigated ?? unmitigated;
  if (!primary) return null;

  const primaryData = extract(primary);
  const max = Math.max(
    ...primaryData,
    ...(unmitigated ? extract(unmitigated) : []),
  );
  const sc = pickScale(max);

  const series: Series[] = [];
  const rawBySeries: number[][] = [];
  let mitigatedSeriesIndex = 0;
  if (hasBoth) {
    const unmitRaw = extract(unmitigated);
    series.push({
      data: scale(unmitRaw, sc.divisor),
      color: UNMITIGATED_COLOR,
      dashed: true,
      legend: "Unmitigated",
    });
    rawBySeries.push(unmitRaw);
    const mitRaw = extract(mitigated);
    series.push({
      data: scale(mitRaw, sc.divisor),
      strokeWidth: 2,
      legend: "Mitigated",
    });
    rawBySeries.push(mitRaw);
    mitigatedSeriesIndex = 1;
  } else {
    series.push({
      data: scale(primaryData, sc.divisor),
      strokeWidth: 2,
      legend: "Unmitigated",
    });
    rawBySeries.push(primaryData);
  }

  return {
    series,
    xLabels: xLabels(primary),
    scale: sc,
    areaSections: buildAreaSections(primary, mitigatedSeriesIndex, showAreaLegend),
    rawBySeries,
  };
}

// --- active-mitigation windows ---------------------------------------------

const SECTION_LABEL_STYLE = { fontSize: 14 };

function buildAreaSections(
  rows: OutputItemGrouped[],
  seriesIndex: number,
  showLegend: boolean,
): AreaSection[] {
  const legend: AreaSection["legend"] = showLegend ? "below" : false;
  const sections: AreaSection[] = [];
  const times = rows.map((r) => r.time);

  // x-index for a given day (nearest timestep)
  const atDay = (day: number): number => {
    let best = 0;
    let bestDiff = Infinity;
    for (let i = 0; i < times.length; i++) {
      const d = Math.abs(times[i] - day);
      if (d < bestDiff) {
        bestDiff = d;
        best = i;
      }
    }
    return best;
  };

  if (
    params.vaccine_enabled &&
    params.vaccine_administration_rate > 0 &&
    params.vaccine_doses_available > 0
  ) {
    const campaignDays =
      params.vaccine_doses_available / params.vaccine_administration_rate;
    const startDay = params.vaccine_start + params.vaccine_ramp_up;
    const endDay = Math.min(params.days, startDay + campaignDays);
    if (endDay > startDay) {
      sections.push({
        seriesIndex,
        startIndex: atDay(startDay),
        endIndex: atDay(endDay),
        color: "var(--accent)",
        opacity: 0.18,
        label: `Day ${Math.round(startDay)}–${Math.round(endDay)}`,
        description: `${formatDoses(params.vaccine_doses_available)} vaccines administered`,
        legend,
        inlineLabelStyle: SECTION_LABEL_STYLE,
        inlineDescriptionStyle: SECTION_LABEL_STYLE,
      });
    }
  }

  if (params.community_enabled && params.community_duration > 0) {
    const end = Math.min(
      params.days,
      params.community_start + params.community_duration,
    );
    sections.push({
      seriesIndex,
      startIndex: atDay(params.community_start),
      endIndex: atDay(end),
      color: "#f59e0b",
      opacity: 0.15,
      label: `Day ${Math.round(params.community_start)}–${Math.round(end)}`,
      description: "Community mitigation",
      legend,
      inlineLabelStyle: SECTION_LABEL_STYLE,
      inlineDescriptionStyle: SECTION_LABEL_STYLE,
    });
  }

  return sections;
}

function formatDoses(n: number): string {
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)}M`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(0)}K`;
  return String(n);
}

// --- derived chart data per panel -----------------------------------------

const overallChart = computed(() =>
  buildChart("InfectionIncidence", sumSeries, true),
);
const deathChart = computed(() => buildChart("DeathIncidence", sumSeries));
const hospChart = computed(() => buildChart("HospitalIncidence", sumSeries));
const symptomaticChart = computed(() =>
  buildChart("SymptomaticIncidence", sumSeries),
);

const groupCharts = computed(() => {
  const labels = params.population_fraction_labels;
  return labels.map((label, i) => ({
    label,
    data: buildChart("InfectionIncidence", (rows) => groupSeries(rows, i)),
  }));
});

// Subtitle based on active mitigations
const subtitle = computed(() => {
  const active: string[] = [];
  if (params.vaccine_enabled) active.push("vaccination");
  if (params.antivirals_enabled) active.push("antivirals");
  if (params.community_enabled) active.push("community mitigation");
  if (params.ttiq_enabled) active.push("TTIQ");
  const pop = formatDoses(params.population);
  if (active.length === 0) {
    return `Baseline scenario: population ${pop} over ${params.days} days`;
  }
  return `Impact of ${active.join(" + ")} on a population of ${pop} over ${params.days} days`;
});

const headerTitle = computed(() =>
  results.value && results.value.mitigation_types.length > 1
    ? "Mitigated vs. Unmitigated Scenario"
    : "Unmitigated Scenario",
);
</script>

<template>
  <div class="results-layout">
    <div ref="root" class="results" id="results-root">
      <!-- Hidden, not removed, on Detection: the PDF report reads its title
           and subtitle from here. -->
      <header v-show="tab !== 'detection'" class="results__header">
        <h1>{{ headerTitle }}</h1>
        <p class="results__subtitle">{{ subtitle }}</p>
      </header>

      <p v-if="error" class="results__error">Error: {{ error }}</p>
      <p v-else-if="!results && running" class="results__loading">Running model…</p>

      <!-- Inactive panels are clipped, not removed: the PDF report reads
           charts and tables from the DOM. -->
      <div v-if="overallChart" class="results__panels">
        <div class="results__panel" :inert="tab !== 'charts'">
          <section class="results__section results__section--flush" id="charts">
            <h2>Overall Infection Incidence</h2>
            <ChartPanel
              :data="overallChart"
              :y-label="`Incidence${overallChart.scale.unit ? ` (${overallChart.scale.unit})` : ''}`"
              filename="overall-infection-incidence"
              :height="320"
            />
          </section>

          <section class="results__grid-3">
            <div v-if="deathChart" class="results__small">
              <h3>Deaths</h3>
              <ChartPanel
                :data="deathChart"
                :y-label="`Incidence${deathChart.scale.unit ? ` (${deathChart.scale.unit})` : ''}`"
                filename="death-incidence"
                :height="180"
              />
            </div>
            <div v-if="hospChart" class="results__small">
              <h3>Hospitalizations</h3>
              <ChartPanel
                :data="hospChart"
                filename="hospital-incidence"
                :height="180"
              />
            </div>
            <div v-if="symptomaticChart" class="results__small">
              <h3>Symptomatic Infections</h3>
              <ChartPanel
                :data="symptomaticChart"
                filename="symptomatic-incidence"
                :height="180"
              />
            </div>
          </section>

          <section class="results__section">
            <h1>Infection Incidence by Age Group</h1>
            <div class="results__grid-n">
              <div v-for="(g, gi) in groupCharts" :key="g.label" class="results__small">
                <h3>{{ g.label }}</h3>
                <ChartPanel
                  v-if="g.data"
                  :data="g.data"
                  :y-label="gi === 0 && g.data.scale.unit ? `(${g.data.scale.unit})` : ''"
                  :filename="`infection-incidence-${g.label.toLowerCase().replace(/\\s+/g, '-')}`"
                  :height="200"
                />
              </div>
            </div>
          </section>
        </div>

        <div class="results__panel" :inert="tab !== 'summary'">
          <section class="results__section" id="summary">
            <SummaryView />
          </section>
        </div>

        <div class="results__panel" :inert="tab !== 'detection'">
          <DetectionSection :results="results" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.results-layout {
  container-type: inline-size;
  container-name: results;
}
.results {
  --results-gap: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: var(--results-gap);
  padding: 1rem 0 1rem 1.5rem;
  min-width: 0;
  /* The scroll container's top padding, so scrollIntoView lands at the top. */
  scroll-margin-top: var(--space-6);
}
.results__panel {
  display: flex;
  flex-direction: column;
  gap: var(--results-gap);
  min-width: 0;
}
/* Not display: none or visibility: hidden. Charts need a real width to
   render, and the report bakes computed visibility into each chart image. */
.results__panel[inert] {
  height: 0;
  overflow: hidden;
}
.results :deep(h1) { font-size: 1.5rem; margin: 0 0 0.5rem; }
.results :deep(h2) { font-size: 1rem; margin: 0 0 0.5rem; }
.results :deep(h3) { font-size: 1rem; margin: 0 0 0.25rem; }
.results__subtitle { margin: 0.25rem 0 0; opacity: 0.65; }
/* The theme's base `section` margin stacks on the flex gap. Drop it so the
   overall chart sits closer to the small charts that continue it. */
.results__section--flush { margin-bottom: 0; }
.results__error { color: #ef4444; }
.results__loading { opacity: 0.7; }
.results__grid-3 {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 1rem;
}
.results__grid-n {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 1rem;
}
@container results (max-width: 900px) {
  .results {
    padding-block: 0.5rem;
    padding-inline-start: 0.75rem;
    --results-gap: 1rem;
  }
  .results :deep(h1) { font-size: 1.25rem; }
  .results__grid-3 {
    grid-template-columns: 1fr;
  }
  .results__grid-n {
    grid-template-columns: 1fr;
  }
}
@container results (min-width: 901px) and (max-width: 1099px) {
  .results__grid-3 {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
