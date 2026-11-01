<script setup lang="ts">
import { ref } from "vue";
import { Button, Icon, Menu } from "cfasim-ui/components";
import type { MenuOption } from "cfasim-ui/components";
import { useParams } from "../composables/useParams";

const emit = defineEmits<{ error: [message: string | null] }>();

const { exportParams, importParams } = useParams();

const fileInput = ref<HTMLInputElement | null>(null);

const items: MenuOption[] = [
  { value: "import", label: "Import parameters…" },
  { value: "export", label: "Export parameters" },
];

function timestamp(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return (
    `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}` +
    `-${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}`
  );
}

function handleExport() {
  const json = JSON.stringify(exportParams(), null, 2);
  const blob = new Blob([json], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `flu-params-${timestamp()}.json`;
  a.click();
  URL.revokeObjectURL(url);
}

function onSelect(item: MenuOption) {
  if (item.value === "import") {
    fileInput.value?.click();
  } else if (item.value === "export") {
    handleExport();
  }
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
  <Menu
    :items="items"
    label="File"
    align="start"
    @select="onSelect"
  >
    <template #trigger>
      <Button variant="neutral" class="file-menu__trigger">
        File
        <Icon icon="keyboard_arrow_down" :size="16" decorative />
      </Button>
    </template>
  </Menu>
  <input
    ref="fileInput"
    type="file"
    accept=".json,application/json"
    hidden
    @change="onFileChosen"
  />
</template>

<style scoped>
.file-menu__trigger {
  gap: var(--space-1);
  padding-right: var(--space-2);
}
</style>
