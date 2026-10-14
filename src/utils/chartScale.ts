import type { Series, AreaSection, GridStyle } from "cfasim-ui/charts";

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

// cfasim-ui pins the y label to the left edge and reserves a fixed 56px
// gutter, which leaves a wide gap next to short tick labels. Negative extra
// padding pulls the plot (and its ticks) in toward the label.
export const Y_LABEL_CHART_PADDING = { left: -6 };

export const TICK_LABEL_STYLE = { fontSize: 12 };

// Matches dynode-web's grid: a 2px dash, 2px gap, #ddd on white. Expressed
// as currentColor at low opacity so it also reads on the dark theme.
export const Y_GRID_STYLE: GridStyle = { lineDash: "2 2", opacity: 0.15 };
