import { parse } from "smol-toml";
import rawToml from "./ui-params.toml?raw";

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

// Flatten nested TOML to dotted-path lookup: { "scenario.days": {...}, ... }
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

const config = flatten(parse(rawToml) as Record<string, unknown>);

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
