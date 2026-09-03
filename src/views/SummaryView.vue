<script setup lang="ts">
import { computed } from "vue";
import { useParams, type ModelOutputExport, type OutputItemGrouped, type OutputTypeLabel } from "../composables/useParams";
import { useModelRun } from "../composables/useModelRun";

const { params } = useParams();
const { results } = useModelRun();

interface Row {
  label: string;
  unmitigated: number;
  mitigated: number | null;
  prevented: number | null;
  preventedPct: number | null;
}

interface Table {
  title: string;
  kind: OutputTypeLabel;
  rows: Row[];
  hasMitigated: boolean;
}

// Sum all per-step incidence values for a given group index (or all groups).
function sumGroup(rows: OutputItemGrouped[], i: number | null): number {
  let total = 0;
  for (const r of rows) {
    if (i === null) {
      for (const v of r.grouped_values) total += v;
    } else {
      total += r.grouped_values[i] ?? 0;
    }
  }
  return total;
}

function buildTable(title: string, kind: OutputTypeLabel, r: ModelOutputExport): Table {
  const unmit = r.output.Unmitigated?.[kind];
  const mit = r.output.Mitigated?.[kind];
  const labels = params.population_fraction_labels;
  const hasMitigated = !!mit;

  const makeRow = (label: string, gi: number | null): Row => {
    const u = unmit ? sumGroup(unmit, gi) : 0;
    const m = mit ? sumGroup(mit, gi) : null;
    const prevented = m !== null ? u - m : null;
    const preventedPct = prevented !== null && u > 0 ? prevented / u : null;
    return {
      label,
      unmitigated: u,
      mitigated: m,
      prevented,
      preventedPct,
    };
  };

  const rows: Row[] = [makeRow("All", null)];
  labels.forEach((lbl, i) => rows.push(makeRow(lbl, i)));
  return { title, kind, rows, hasMitigated };
}

const tables = computed<Table[]>(() => {
  const r = results.value;
  if (!r) return [];
  return [
    buildTable("Infections", "InfectionIncidence", r),
    buildTable("Symptomatic Infections", "SymptomaticIncidence", r),
    buildTable("Hospitalizations", "HospitalIncidence", r),
    buildTable("Deaths", "DeathIncidence", r),
  ];
});

const fmt = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });
function roundTo1000(n: number): number {
  return Math.round(n / 1000) * 1000;
}
function formatCount(n: number): string {
  return fmt.format(roundTo1000(n));
}

const pctFmt = new Intl.NumberFormat("en-US", {
  style: "percent",
  maximumFractionDigits: 1,
});
function formatPct(n: number): string {
  return pctFmt.format(n);
}
</script>

<template>
  <div v-if="tables.length" class="summary">
    <section v-for="t in tables" :key="t.kind" class="summary__section">
      <h3 class="summary__title">{{ t.title }}</h3>
      <div class="summary__scroll">
        <table class="summary__table">
          <thead>
            <tr>
              <th class="summary__col-group">Age group</th>
              <th>Unmitigated</th>
              <th v-if="t.hasMitigated">Mitigated</th>
              <th v-if="t.hasMitigated">Prevented</th>
              <th v-if="t.hasMitigated">% Prevented</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in t.rows" :key="row.label">
              <td class="summary__label">{{ row.label }}</td>
              <td class="summary__num">{{ formatCount(row.unmitigated) }}</td>
              <td v-if="t.hasMitigated" class="summary__num">
                {{ row.mitigated !== null ? formatCount(row.mitigated) : "—" }}
              </td>
              <td v-if="t.hasMitigated" class="summary__num">
                {{ row.prevented !== null ? formatCount(row.prevented) : "—" }}
              </td>
              <td v-if="t.hasMitigated" class="summary__prevented">
                <div class="summary__prevented-value">
                  {{ row.preventedPct !== null ? formatPct(row.preventedPct) : "—" }}
                </div>
                <div v-if="row.preventedPct !== null" class="summary__bar-track">
                  <div
                    v-if="row.preventedPct > 0"
                    class="summary__bar"
                    :style="{ width: `${Math.min(100, row.preventedPct * 100)}%` }"
                  />
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>

<style scoped>
.summary {
  display: flex;
  flex-direction: column;
  gap: 2rem;
}
.summary__title {
  font-size: 1rem;
  font-weight: 700;
  margin: 0 0 0.5rem;
}
.summary__scroll {
  overflow-x: auto;
  scrollbar-width: thin;
}
.summary__table {
  width: 100%;
  min-width: 480px;
  border-collapse: collapse;
  font-variant-numeric: tabular-nums;
}
.summary__table th {
  text-align: left;
  font-size: 0.7rem;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  white-space: nowrap;
  opacity: 0.55;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid rgba(128, 128, 128, 0.25);
}
.summary__col-group {
  width: 200px;
  max-width: 200px;
}
.summary__table th:not(.summary__col-group) {
  width: auto;
}
.summary__table td {
  padding: 0.6rem 0.75rem;
  border-bottom: 1px solid rgba(128, 128, 128, 0.12);
}
.summary__label {
  font-weight: 500;
}
.summary__num {
  font-feature-settings: "tnum";
}
.summary__prevented {
  position: relative;
}
.summary__prevented-value {
  font-feature-settings: "tnum";
}
.summary__bar-track {
  margin-top: 0.25rem;
  height: 3px;
  background: rgba(128, 128, 128, 0.2);
  border-radius: 2px;
  overflow: hidden;
}
.summary__bar {
  height: 100%;
  background: var(--accent);
  border-radius: 2px;
}
</style>
