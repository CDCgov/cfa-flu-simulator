import type { Series, AreaSection } from "cfasim-ui/charts";

export interface Scale {
  divisor: number;
  unit: string;
}

export interface ChartData {
  series: Series[];
  xLabels: string[];
  scale: Scale;
  areaSections: AreaSection[];
  rawBySeries: number[][];
}

export function pickScale(maxValue: number): Scale {
  if (maxValue >= 1e6) return { divisor: 1e6, unit: "Millions" };
  if (maxValue >= 1e3) return { divisor: 1e3, unit: "Thousands" };
  return { divisor: 1, unit: "" };
}

export function scale(data: number[], divisor: number): number[] {
  return divisor === 1 ? data : data.map((v) => v / divisor);
}

export interface CsvColumn {
  header: string;
  data: number[];
}

// One row per x label, using raw counts rather than display-scaled values.
export function chartCsv(xLabels: string[], columns: CsvColumn[]): string {
  const headers = ["day", ...columns.map((c) => c.header)];
  const rows = xLabels.map((day, r) =>
    [day, ...columns.map((c) => String(c.data[r] ?? ""))].join(","),
  );
  return [headers.join(","), ...rows].join("\n");
}
