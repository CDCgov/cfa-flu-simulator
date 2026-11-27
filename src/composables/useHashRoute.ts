import { onMounted, onUnmounted, ref } from "vue";
import { RESULTS_TABS, type ResultsTab } from "../views/ResultsView.vue";

const DEFAULT_TAB: ResultsTab = "charts";
// The tab whose content has linkable sections (the docs/model.md headings).
const SECTION_TAB: ResultsTab = "model";

export interface HashRoute {
  tab: ResultsTab;
  /** Heading id inside the model description, or null for the top of a tab. */
  section: string | null;
}

// The hash is either a tab name or a model description heading id. Tab
// names win, so a heading must not slugify to one of them.
export function parseHash(hash: string): HashRoute {
  const id = hash.replace(/^#/, "");
  if (id === "") return { tab: DEFAULT_TAB, section: null };
  const tab = RESULTS_TABS.find((name) => name === id);
  if (tab) return { tab, section: null };
  return { tab: SECTION_TAB, section: id };
}

export function formatHash(route: HashRoute): string {
  if (route.section) return `#${route.section}`;
  return route.tab === DEFAULT_TAB ? "" : `#${route.tab}`;
}

/**
 * Keeps the results tab and the model description section in the URL hash.
 * The query string belongs to useUrlParams, which rewrites it wholesale but
 * preserves the hash. Like it, this replaces the history entry: a pushed
 * entry would hold an older query string, and going back to it would revert
 * parameter edits made since.
 */
export function useHashRoute() {
  const initial = parseHash(window.location.hash);
  const tab = ref<ResultsTab>(initial.tab);
  const section = ref<string | null>(initial.section);

  function navigate(route: HashRoute) {
    tab.value = route.tab;
    section.value = route.section;
    const url =
      window.location.pathname + window.location.search + formatHash(route);
    window.history.replaceState(window.history.state, "", url);
  }

  // Hash edited in the address bar, or back/forward across such an edit.
  // popstate as well: browsers skip hashchange when the two entries also
  // differ in their query string.
  function syncFromLocation() {
    const route = parseHash(window.location.hash);
    tab.value = route.tab;
    section.value = route.section;
  }

  onMounted(() => {
    window.addEventListener("hashchange", syncFromLocation);
    window.addEventListener("popstate", syncFromLocation);
  });
  onUnmounted(() => {
    window.removeEventListener("hashchange", syncFromLocation);
    window.removeEventListener("popstate", syncFromLocation);
  });

  return {
    tab,
    section,
    selectTab: (next: ResultsTab) => navigate({ tab: next, section: null }),
    selectSection: (id: string) => navigate({ tab: SECTION_TAB, section: id }),
  };
}
