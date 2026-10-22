import { parse } from "smol-toml";
import rawToml from "./ui-params.toml?raw";
import type { Parameters } from "../composables/useParams";

export type FieldType = "integer" | "float" | "percent" | "select";

export interface SelectOption {
  value: number;
  label: string;
}

export interface FieldConfig {
  section?: string;
  show_when_doses_2?: boolean;
  label: string;
  tooltip?: string;
  // Path of the 2-dose companion rendered as this field's upper handle.
  range_with?: string;
  // Used when paired with the 2-dose companion on one two-handle slider.
  range_label?: string;
  range_tooltip?: string;
  min?: number;
  max?: number;
  step?: number;
  default?: number;
  type?: FieldType;
  slider?: boolean;
  per_group?: boolean;
  matrix?: boolean;
  options?: SelectOption[];
}

// Flatten parameter tables to dotted-path lookup. Tables without a `label`
// are retained in the TOML solely as non-visible runtime defaults.
function flatten(
  obj: Record<string, unknown>,
  prefix = "",
  out: Record<string, FieldConfig> = {},
): Record<string, FieldConfig> {
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value && typeof value === "object" && !Array.isArray(value)) {
      const v = value as Record<string, unknown>;
      // Leaf: has `label` (every field config does).
      if (typeof v.label === "string") {
        out[path] = v as unknown as FieldConfig;
      } else {
        flatten(v, path, out);
      }
    }
  }
  return out;
}

const parsedToml = parse(rawToml) as Record<string, unknown>;
const config = flatten(parsedToml);

export async function loadDefaultParameters(): Promise<Parameters> {
  const base = (import.meta.env.BASE_URL ?? "/").replace(/\/$/, "");
  const response = await fetch(`${base}/ui-params.toml`, { cache: "no-store" });
  if (!response.ok) {
    throw new Error(`Unable to load ui-params.toml (${response.status})`);
  }
  const parsed = parse(await response.text()) as Record<string, unknown>;
  const defaults = Object.fromEntries(
    Object.entries(parsed).flatMap(([key, value]) => {
      if (
        value &&
        typeof value === "object" &&
        !Array.isArray(value) &&
        "default" in value
      ) {
        return [[key, (value as Record<string, unknown>).default]];
      }
      return [];
    }),
  );
  if (Object.keys(defaults).length === 0) {
    throw new Error("ui-params.toml: no parameter defaults found");
  }
  return structuredClone(defaults) as unknown as Parameters;
}

export function getField(path: string): FieldConfig {
  const entry = config[path];
  if (!entry) throw new Error(`ui-params.toml: no config for "${path}"`);
  return entry;
}

export function allFields(): Record<string, FieldConfig> {
  return config;
}

// [lower, upper] paths for fields rendered as one two-handle slider.
export function rangePairs(): [string, string][] {
  return Object.entries(config)
    .filter(([, cfg]) => cfg.range_with)
    .map(([path, cfg]) => [path, cfg.range_with as string]);
}

// Both handles share one scale, so the paired configs have to agree. Checked
// once here rather than during render, where a throw would blank the app.
for (const [lower, upper] of rangePairs()) {
  const a = getField(lower);
  const b = getField(upper);
  for (const key of ["min", "max", "step", "type", "slider"] as const) {
    if (a[key] !== b[key]) {
      throw new Error(
        `ui-params.toml: paired "${lower}"/"${upper}" disagree on "${key}"`,
      );
    }
  }
}

// Fields belonging to a section, in TOML declaration order.
export function fieldsInSection(section: string): [string, FieldConfig][] {
  return Object.entries(config).filter(([, cfg]) => cfg.section === section);
}
