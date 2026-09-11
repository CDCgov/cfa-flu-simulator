<script setup lang="ts">
import { ref, type ComponentPublicInstance } from "vue";
import { Button, Icon } from "cfasim-ui/components";
import { useParams } from "../composables/useParams";

const emit = defineEmits<{ error: [message: string | null] }>();

const { reset, exportParams, importParams } = useParams();

const open = ref(false);
const toggle = ref<ComponentPublicInstance | null>(null);
const list = ref<HTMLElement | null>(null);
const fileInput = ref<HTMLInputElement | null>(null);

function close() {
  list.value?.hidePopover();
}

// Popovers render in the top layer, where CSS can't position them relative
// to the toggle, so place the list from the toggle's rect when it opens.
function onToggle(event: ToggleEvent) {
  open.value = event.newState === "open";
  const button = toggle.value?.$el as HTMLElement | undefined;
  if (!open.value || !button || !list.value) return;
  const rect = button.getBoundingClientRect();
  list.value.style.top = `${rect.bottom}px`;
  list.value.style.left = `${rect.left}px`;
}

function timestamp(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return (
    `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}` +
    `-${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}`
  );
}

function handleReset() {
  close();
  emit("error", null);
  reset();
}

function handleExport() {
  close();
  const json = JSON.stringify(exportParams(), null, 2);
  const blob = new Blob([json], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `flu-params-${timestamp()}.json`;
  a.click();
  URL.revokeObjectURL(url);
}

function handleImport() {
  close();
  fileInput.value?.click();
}

async function onFileChosen(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  try {
    importParams(JSON.parse(await file.text()));
    emit("error", null);
  } catch (err) {
    const detail = err instanceof Error ? err.message : String(err);
    emit("error", `Could not import ${file.name}: ${detail}`);
  }
}
</script>

<template>
  <div class="params-menu">
    <Button
      ref="toggle"
      variant="secondary"
      popovertarget="params-menu-list"
      :aria-expanded="open"
    >
      Parameters
      <Icon icon="keyboard_arrow_down" size="sm" decorative />
    </Button>
    <div
      id="params-menu-list"
      ref="list"
      class="params-menu__list"
      popover
      aria-label="Parameter actions"
      @toggle="onToggle"
    >
      <button type="button" @click="handleReset">Reset to defaults</button>
      <button type="button" @click="handleImport">Import from JSON…</button>
      <button type="button" @click="handleExport">Export to JSON</button>
    </div>
    <input
      ref="fileInput"
      type="file"
      accept=".json,application/json"
      hidden
      @change="onFileChosen"
    />
  </div>
</template>

<style scoped>
.params-menu {
  display: inline-block;
}

/* The cfasim-ui Button sets no gap and pads both sides equally, which leaves
   the caret flush against the label and floating well short of the edge. */
.params-menu .button {
  gap: var(--space-1);
  padding-right: var(--space-2);
}

/* The theme's --shadow-focus is an invalid light-dark() of shadows, so the
   cfasim-ui Button has no visible focus ring; draw an outline instead. */
.params-menu .button:focus-visible,
.params-menu__list button:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: 2px;
}

.params-menu__list {
  position: fixed;
  inset: auto;
  margin: var(--space-1) 0 0;
  min-width: 11rem;
  padding: var(--space-1);
  flex-direction: column;
  background: var(--color-bg-0);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  color: var(--color-text);
}

.params-menu__list:popover-open {
  display: flex;
}

.params-menu__list button {
  all: unset;
  display: block;
  box-sizing: border-box;
  width: 100%;
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
  color: var(--color-text);
  cursor: pointer;
  white-space: nowrap;
}

.params-menu__list button:hover,
.params-menu__list button:focus-visible {
  background: var(--color-bg-2);
}
</style>
