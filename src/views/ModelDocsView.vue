<script setup lang="ts">
import "katex/dist/katex.min.css";
import { onMounted, ref, watch } from "vue";
import modelDocs from "../../docs/model.md";

const props = defineProps<{ section: string | null }>();
const emit = defineEmits<{ navigate: [section: string] }>();

const article = ref<HTMLElement | null>(null);

// An unknown id falls back to the top of the document.
function showSection(id: string | null) {
  const root = article.value;
  if (!root) return;
  const heading = id
    ? root.querySelector<HTMLElement>(`#${CSS.escape(id)}`)
    : null;
  if (!heading) {
    root.scrollIntoView({ block: "start" });
    return;
  }
  heading.scrollIntoView({ block: "start" });
  heading.focus({ preventScroll: true });
}

onMounted(() => showSection(props.section));
watch(() => props.section, showSection);

// In-page links update the URL through the app instead of pushing a history
// entry. Modified clicks keep their browser meaning (open in a new tab).
function handleClick(event: MouseEvent) {
  if (event.button !== 0) return;
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
  const link = (event.target as Element).closest('a[href^="#"]');
  const id = link?.getAttribute("href")?.slice(1);
  if (!id) return;
  event.preventDefault();
  emit("navigate", id);
  // The watcher does not fire when the link is to the current section.
  showSection(id);
}
</script>

<template>
  <article ref="article" class="model-docs" @click="handleClick" v-html="modelDocs"></article>
</template>

<style scoped>
.model-docs {
  max-width: 48rem;
  padding: 1rem 0 2rem;
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

/* Leaves the gap the scroll container's top padding gives the first line. */
.model-docs,
.model-docs :deep(:is(h1, h2, h3, h4)) {
  scroll-margin-top: var(--space-6);
}
/* Headings take focus when linked to, only to move the keyboard position. */
.model-docs :deep(:is(h1, h2, h3, h4):focus) {
  outline: none;
}

.model-docs :deep(.header-anchor) {
  margin-left: 0.4em;
  font-weight: 400;
  text-decoration: none;
  opacity: 0;
}
.model-docs :deep(.header-anchor)::before {
  content: "#";
}
.model-docs :deep(:is(h2, h3, h4):hover .header-anchor),
.model-docs :deep(.header-anchor:focus-visible) {
  opacity: 1;
}
/* No hover on touch screens, so the links stay faintly visible. */
@media (hover: none) {
  .model-docs :deep(.header-anchor) {
    opacity: 0.5;
  }
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
  .model-docs :deep(.header-anchor) {
    display: none;
  }
  /* The references are links; print where they go. */
  .model-docs :deep(a[href^="http"])::after {
    content: " (" attr(href) ")";
    font-size: 0.85em;
    word-break: break-all;
  }
}
</style>
