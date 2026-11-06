<script setup lang="ts">
import "katex/dist/katex.min.css";
import modelDocs from "../../docs/model.md";
</script>

<template>
  <article class="model-docs" v-html="modelDocs"></article>
</template>

<style scoped>
.model-docs {
  max-width: 48rem;
  padding: 1rem 0 2rem 1.5rem;
}

/* The theme gives headings a bottom margin only; running text needs space
   above them too. h1 matches the results header, h2 opens a new part. */
.model-docs :deep(h1) {
  font-size: 1.5rem;
}
.model-docs :deep(h2) {
  font-size: 1.25rem;
  margin-top: 2rem;
  padding-top: 1rem;
  border-top: 1px solid var(--color-border);
}
.model-docs :deep(h3) {
  margin-top: 1.5rem;
}
.model-docs :deep(h4) {
  margin-top: 1.25rem;
}

/* Wide equations scroll sideways. overflow-x alone would also add a vertical
   scrollbar when a tall fraction overflows by a pixel; the padding keeps
   overflow-y: hidden from clipping it. */
.model-docs :deep(.katex-display) {
  overflow-x: auto;
  overflow-y: hidden;
  padding-block: 0.25rem;
}

@media print {
  .model-docs {
    max-width: none;
    padding: 0;
  }
  .model-docs :deep(h2),
  .model-docs :deep(h3),
  .model-docs :deep(h4) {
    break-after: avoid;
  }
  .model-docs :deep(.katex-display) {
    break-inside: avoid;
    overflow: visible;
  }
  /* The references are links; print where they go. */
  .model-docs :deep(a[href^="http"])::after {
    content: " (" attr(href) ")";
    font-size: 0.85em;
    word-break: break-all;
  }
}
</style>
