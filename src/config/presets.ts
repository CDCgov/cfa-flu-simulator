import { parse } from "smol-toml";
import populationToml from "./population-presets.toml?raw";
import viralToml from "./viral-presets.toml?raw";
import type { Parameters } from "../composables/useParams";
import { DEFAULT_PARAMETERS } from "./uiConfig";

// Preset fields are numbers, or one number per age group.
type PresetValue = number | number[];
type PresetValues = Record<string, PresetValue>;

export interface Preset {
  id: string;
  label: string;
  params: PresetValues;
}

// A group's presets fix every field under its "Advanced settings", so
// changing any one of them reads as leaving the preset.
const POPULATION_FIELDS = [
  "population",
  "initial_infections",
  "fraction_initial_immune",
];

const VIRAL_FIELDS = [
  "r0",
  "latent_period",
  "infectious_period",
  "p_symp_given_inf",
  "p_hosp_given_symp",
  "p_death_given_hosp",
];

function loadPresets(
  file: string,
  rawToml: string,
  fields: string[],
): Preset[] {
  const tables = parse(rawToml) as Record<
    string,
    { label: string; params?: PresetValues }
  >;
  const defaults = DEFAULT_PARAMETERS as unknown as PresetValues;
  const presets: Preset[] = [];
  for (const [id, table] of Object.entries(tables)) {
    const overrides = table.params ?? {};
    for (const key of Object.keys(overrides)) {
      // A misspelt key would otherwise be ignored without a trace.
      if (!fields.includes(key)) {
        throw new Error(`${file}: "${id}" sets unknown field "${key}"`);
      }
    }
    // Fields a preset leaves out are held at their defaults.
    const params: PresetValues = {};
    for (const key of fields) {
      params[key] = overrides[key] ?? defaults[key];
    }
    presets.push({ id, label: table.label, params });
  }
  return presets;
}

export const POPULATION_PRESETS = loadPresets(
  "population-presets.toml",
  populationToml,
  POPULATION_FIELDS,
);

export const VIRAL_PRESETS = loadPresets(
  "viral-presets.toml",
  viralToml,
  VIRAL_FIELDS,
);

// Values that went through the URL or a slider can differ from the TOML
// literal by float noise.
const TOLERANCE = 1e-9;

function sameValue(a: PresetValue, b: PresetValue): boolean {
  const left = [a].flat();
  const right = [b].flat();
  return (
    left.length === right.length &&
    left.every((value, i) => Math.abs(value - right[i]) <= TOLERANCE)
  );
}

/** The preset whose values all equal the current parameters, if any. */
export function findMatchingPreset(
  presets: Preset[],
  params: Parameters,
): Preset | undefined {
  const current = params as unknown as PresetValues;
  return presets.find((preset) =>
    Object.entries(preset.params).every(([key, value]) =>
      sameValue(current[key], value),
    ),
  );
}
