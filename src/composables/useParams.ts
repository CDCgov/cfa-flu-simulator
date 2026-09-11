import {
  inject,
  provide,
  reactive,
  ref,
  shallowRef,
  toRaw,
  watch,
  type InjectionKey,
  type Ref,
} from "vue";
import { parse } from "smol-toml";
import { useUrlParams } from "cfasim-ui/shared";
import { allFields, rangePairs } from "../config/uiConfig";
import rawDefaults from "../../model/default-params.toml?raw";

// Flat schema mirrors the Rust `Parameters` struct (wasm boundary).
// Mitigation fields are prefixed (vaccine_, antivirals_, community_, ttiq_)
// so the whole object round-trips through URL query strings.
export interface Parameters {
  n: number;
  days: number;
  population: number;
  population_fraction_labels: string[];
  population_fractions: number[];
  contact_matrix: number[];
  initial_infections: number;
  fraction_initial_immune: number;
  r0: number;
  latent_period: number;
  infectious_period: number;
  fraction_symptomatic: number[];
  fraction_hospitalized: number[];
  hospitalization_delay: number;
  fraction_dead: number[];
  death_delay: number;
  p_test_sympto: number;
  test_sensitivity: number;
  p_test_forward: number;

  vaccine_enabled: boolean;
  vaccine_editable: boolean;
  vaccine_doses: number;
  vaccine_start: number;
  vaccine_dose2_delay: number;
  vaccine_p_get_2_doses: number;
  vaccine_administration_rate: number;
  vaccine_doses_available: number;
  vaccine_ramp_up: number;
  vaccine_ve_s: number;
  vaccine_ve_i: number;
  vaccine_ve_p: number;
  vaccine_ve_2s: number;
  vaccine_ve_2i: number;
  vaccine_ve_2p: number;

  antivirals_enabled: boolean;
  antivirals_editable: boolean;
  antivirals_fraction_adhere: number;
  antivirals_fraction_diagnosed_prescribed_inpatient: number;
  antivirals_fraction_diagnosed_prescribed_outpatient: number;
  antivirals_fraction_seek_care: number;
  antivirals_ave_i: number;
  antivirals_ave_p_hosp: number;
  antivirals_ave_p_death: number;

  community_enabled: boolean;
  community_editable: boolean;
  community_start: number;
  community_duration: number;
  community_effectiveness: number[];

  ttiq_enabled: boolean;
  ttiq_editable: boolean;
  ttiq_p_id_infectious: number;
  ttiq_p_infectious_isolates: number;
  ttiq_isolation_reduction: number;
  ttiq_p_contact_trace: number;
  ttiq_p_traced_quarantines: number;
}

export type MitigationLabel = "Unmitigated" | "Mitigated";
export type OutputTypeLabel =
  | "InfectionIncidence"
  | "SymptomaticIncidence"
  | "HospitalIncidence"
  | "DeathIncidence";

export interface OutputItemGrouped {
  time: number;
  grouped_values: number[];
}

export interface ModelOutputExport {
  output: Record<MitigationLabel, Record<OutputTypeLabel, OutputItemGrouped[]>>;
  p_detect: Record<MitigationLabel, { time: number; value: number }[]>;
  mitigation_types: MitigationLabel[];
  output_types: OutputTypeLabel[];
}

export interface WasmModel {
  run: () => ModelOutputExport;
  free: () => void;
}

export interface WasmModule {
  default: () => Promise<unknown>;
  get_default_parameters: () => Parameters;
  SEIRModelUnified: new (params: Parameters) => WasmModel;
}

export { loadWasm };

let wasmPromise: Promise<WasmModule> | null = null;

function loadWasm(): Promise<WasmModule> {
  if (!wasmPromise) {
    // Fully-qualified URL so Vite skips its resolver (files under /public/
    // cannot be imported via bare paths from source code).
    const base = (import.meta.env.BASE_URL ?? "/").replace(/\/$/, "") + "/";
    const url = `${window.location.origin}${base}wasm/cfa-flu-simulator/cfa_flu_simulator.js`;
    wasmPromise = (async () => {
      const mod = (await import(/* @vite-ignore */ url)) as WasmModule;
      await mod.default();
      return mod;
    })();
  }
  return wasmPromise;
}

export interface ParamsStore {
  params: Parameters;
  ready: Ref<boolean>;
  reset: () => void;
  /** Plain (non-reactive) copy of the current parameters. */
  exportParams: () => Parameters;
  /**
   * Replace the current parameters with `input` layered over the defaults.
   * Unknown keys and values of the wrong type are dropped. Throws if the
   * input is not an object or contains no recognised parameters.
   */
  importParams: (input: unknown) => void;
}

const ParamsKey: InjectionKey<ParamsStore> = Symbol("params");

// Build-time snapshot of model/default-params.toml (the same file the wasm
// crate embeds via include_str!). Used to seed reactive state before wasm
// loads; overwritten with `get_default_parameters()` on ready.
const TOML_DEFAULTS = parse(rawDefaults) as unknown as Parameters;

function seedParameters(): Parameters {
  return structuredClone(TOML_DEFAULTS);
}

function sameShape(value: unknown, reference: unknown): boolean {
  if (Array.isArray(reference)) {
    return (
      Array.isArray(value) &&
      value.every((v) => typeof v === typeof reference[0])
    );
  }
  return typeof value === typeof reference;
}

export function mergeImported(
  input: unknown,
  defaults: Parameters,
): Parameters {
  if (input === null || typeof input !== "object" || Array.isArray(input)) {
    throw new Error("Parameters file must contain a JSON object");
  }
  const source = input as Record<string, unknown>;
  const merged: Record<string, unknown> = { ...structuredClone(defaults) };
  let accepted = 0;
  for (const key of Object.keys(defaults) as (keyof Parameters)[]) {
    const value = source[key];
    if (value === undefined || !sameShape(value, defaults[key])) continue;
    merged[key] = structuredClone(value);
    accepted++;
  }
  if (accepted === 0) {
    throw new Error("No recognised parameters found in file");
  }
  return merged as unknown as Parameters;
}

// Keys omitted from URL sync: structural/UI-only fields the user never edits.
const URL_IGNORE: (keyof Parameters)[] = [
  "n",
  "population_fraction_labels",
  "vaccine_editable",
  "antivirals_editable",
  "community_editable",
  "ttiq_editable",
];

// Only exposed for a 2-dose vaccine; held at defaults otherwise so a
// single-dose link doesn't carry inert query params.
const TWO_DOSE_ONLY = Object.entries(allFields())
  .filter(([, cfg]) => cfg.show_when_doses_2)
  .map(([path]) => path as keyof Parameters);

// Pairs sharing a two-handle slider. reka-ui sorts handle values, so an
// out-of-order pair would swap on first interaction; the second dose is
// raised to meet the first instead, before it can reach a slider.
const DOSE_PAIRS = rangePairs() as [keyof Parameters, keyof Parameters][];

export function createParamsStore(): ParamsStore {
  const params = reactive<Parameters>(seedParameters());
  const ready = ref(false);
  // shallowRef: defaults are an immutable snapshot. A plain ref() would
  // deep-proxy the object, and useUrlParams' structuredClone chokes on the
  // reactive-wrapped nested arrays.
  const wasmDefaults = shallowRef<Parameters | null>(null);

  const { hydrate, reset: resetUrl } = useUrlParams(
    params,
    () => wasmDefaults.value ?? undefined,
    { ignore: URL_IGNORE },
  );

  // Values parked while the vaccine is single-dose, so the fields can be
  // cleared from the URL without losing what the user typed.
  let stashed: Record<string, unknown> | null = null;

  function normalize() {
    const write = params as Record<string, unknown>;
    if (params.vaccine_doses === 2) {
      if (stashed) {
        Object.assign(write, stashed);
        stashed = null;
      }
      for (const [first, second] of DOSE_PAIRS) {
        const a = params[first] as number;
        if ((params[second] as number) < a) write[second as string] = a;
      }
      return;
    }
    // Guarded: clearing the fields re-triggers the watcher, which would
    // otherwise stash the defaults over the values just saved.
    if (!stashed) {
      stashed = Object.fromEntries(TWO_DOSE_ONLY.map((k) => [k, params[k]]));
    }
    const defaults = wasmDefaults.value ?? TOML_DEFAULTS;
    for (const key of TWO_DOSE_ONLY) {
      write[key as string] = structuredClone(defaults[key]);
    }
  }

  // Watches the fields too, so a URL or imported JSON that sets them
  // alongside a 1-dose vaccine is normalized as well.
  watch(
    () => [
      params.vaccine_doses,
      ...TWO_DOSE_ONLY.map((k) => params[k]),
      ...DOSE_PAIRS.flat().map((k) => params[k]),
    ],
    normalize,
  );

  loadWasm().then((mod) => {
    wasmDefaults.value = mod.get_default_parameters();
    Object.assign(params, wasmDefaults.value);
    hydrate();
    normalize();
    ready.value = true;
  });

  function exportParams(): Parameters {
    return structuredClone(toRaw(params));
  }

  function importParams(input: unknown) {
    const defaults = wasmDefaults.value ?? TOML_DEFAULTS;
    Object.assign(params, mergeImported(input, defaults));
  }

  return {
    params,
    ready,
    reset: () => resetUrl(),
    exportParams,
    importParams,
  };
}

export function provideParams(): ParamsStore {
  const store = createParamsStore();
  provide(ParamsKey, store);
  return store;
}

export function useParams(): ParamsStore {
  const store = inject(ParamsKey);
  if (!store) throw new Error("useParams() called without provideParams()");
  return store;
}
