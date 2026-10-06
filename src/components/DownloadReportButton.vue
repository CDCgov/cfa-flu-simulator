<script setup lang="ts">
import { ref } from "vue";
import { Button, Icon } from "cfasim-ui/components";
import { useParams } from "../composables/useParams";
import { generateReport } from "../utils/pdfReport";

const { params } = useParams();

const downloading = ref(false);
async function handleDownload() {
  const container = document.getElementById("results-root");
  if (!container || downloading.value) return;
  downloading.value = true;
  try {
    await generateReport(container, params);
  } catch (e) {
    console.error("Report generation failed", e);
  } finally {
    downloading.value = false;
  }
}
</script>

<template>
  <Button
    class="download"
    variant="secondary"
    :disabled="downloading"
    @click="handleDownload"
  >
    <Icon icon="download" size="sm" />
    <span class="download__label">
      {{ downloading ? "Generating…" : "Download report" }}
    </span>
  </Button>
</template>

<style scoped>
.download {
  gap: var(--space-1);
}

/* Below the layout's mobile breakpoint the tab bar has no room for the
   label, so collapse to an icon the size of the theme toggle beside it. */
@media (max-width: 767px) {
  .download {
    width: 32px;
    min-height: 32px;
    padding: 0;
  }
  .download__label {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
}
</style>
