<script setup lang="ts">
import { ref } from "vue";
import { SidebarLayout } from "cfasim-ui/components";
import { provideParams } from "./composables/useParams";
import ParamsMenu from "./components/ParamsMenu.vue";
import ScenarioSection from "./sections/ScenarioSection.vue";
import VaccineSection from "./sections/VaccineSection.vue";
import AntiviralsSection from "./sections/AntiviralsSection.vue";
import CommunitySection from "./sections/CommunitySection.vue";
import DownloadReportButton from "./components/DownloadReportButton.vue";
import ResultsView, { type ResultsTab } from "./views/ResultsView.vue";

const { ready } = provideParams();
const importError = ref<string | null>(null);

const tabs: { value: ResultsTab; label: string }[] = [
  { value: "charts", label: "Charts" },
  { value: "summary", label: "Summary" },
];
const tab = ref<ResultsTab>("charts");
</script>

<template>
  <SidebarLayout v-model:tab="tab" :tabs="tabs">
    <template #sidebar>
      <template v-if="ready">
        <div class="toolbar">
          <ParamsMenu @error="importError = $event" />
        </div>
        <p v-if="importError" class="import-error" role="alert">
          {{ importError }}
        </p>
        <section class="sidebar-group">
          <h2>Scenario</h2>
          <ScenarioSection />
        </section>
        <section class="sidebar-group">
          <h2>Interventions</h2>
          <VaccineSection />
          <AntiviralsSection />
          <CommunitySection />
        </section>
      </template>
      <p v-else class="loading">Loading model…</p>
      <footer class="sidebar-footer">
        <a
          href="https://www.cdc.gov/other/privacy.html"
          target="_blank"
          rel="noopener noreferrer"
          >Privacy Policy</a
        >
      </footer>
    </template>
    <template #topbar>
      <DownloadReportButton v-if="ready" />
    </template>
    <ResultsView v-if="ready" :tab="tab" />
  </SidebarLayout>
</template>

<style>
:root {
  --accent: rgb(0, 87, 183);
  --font-family:
    "IBM Plex Sans", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
    Helvetica, Arial, sans-serif;
  /* Input and button colour from cfasim-ui's cdc theme (themes/cdc.css),
     without adopting the rest of that theme. */
  --color-primary: light-dark(#007a99, #33b5d6);
  --color-primary-hover: light-dark(#006680, #4dc4e0);
  --color-primary-active: light-dark(#005c73, #1fa8cc);
}

/* Sidebar background and edge from the cdc theme. SidebarLayout paints
   --color-bg-1 on its scroll pane and collapsed rail with scoped rules, so
   the custom property is overridden on the rail (which both inherit) rather
   than out-specifying those rules. */
.SidebarLayout .SidebarRail {
  --color-bg-1: light-dark(#f4fbfc, #162a2e);
}

.SidebarLayout .SidebarRail .SidebarScroll {
  border-right-color: light-dark(#c5e4ec, #264349);
}

.input-label {
  font-size: var(--font-size-sm);
}
</style>

<style scoped>
.loading {
  padding: 1rem;
  opacity: 0.7;
}

/* Shares the row with the sidebar toggle in the top-right corner. */
.toolbar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-shrink: 0;
  height: var(--toggle-size);
  margin-top: calc(-1 * var(--toggle-size));
  margin-right: var(--toggle-size);
}

/* Heading plus its content share one column so the spacing inside a group
   (12px) is tighter than the spacing between groups (24px). */
.sidebar-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  margin: 0;
}

.sidebar-group + .sidebar-group {
  margin-top: var(--space-3);
}

.sidebar-group:last-of-type {
  padding-bottom: var(--space-3);
}

/* .Sidebar prefix outranks the library's own h2 margin rule. */
.Sidebar .SidebarScroll h2 {
  margin: 0;
}

.import-error {
  margin: 0;
  color: var(--color-error);
  font-size: var(--font-size-xs);
}

.sidebar-footer {
  margin-top: auto;
  padding-top: var(--space-4);
  border-top: 1px solid var(--color-border);
  font-size: var(--font-size-sm);
}

:deep(.SidebarScroll) {
  padding-top: calc(var(--space-4) + var(--space-2));
}

:deep(.MainContent) {
  max-width: 1600px;
}

/* Three tabs plus the two bar buttons overflow the narrowest phones. */
@media (max-width: 389px) {
  :deep(.TabsBar),
  :deep(.TabsTrigger) {
    padding-inline: var(--space-2);
  }
}
</style>
