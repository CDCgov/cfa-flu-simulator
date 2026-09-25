#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["altair==5.3.0", "typing-extensions"]
# ///
"""Compare snapshot timecourses with the same fixtures from another git ref."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

import altair as alt

TIMECOURSES = {
    "infection_incidence": "Infections",
    "symptomatic_incidence": "Symptomatic infections",
    "hospital_incidence": "Hospitalizations",
    "death_incidence": "Deaths",
}


def git_root() -> Path:
    return Path(
        subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"], text=True
        ).strip()
    )


def fixture_names(snapshot_dir: Path) -> list[str]:
    return sorted(path.stem for path in snapshot_dir.glob("*.json"))


def load_fixture(path: Path, ref: str | None = None) -> dict[str, Any]:
    if ref is None:
        return json.loads(path.read_text())

    return json.loads(
        subprocess.check_output(["git", "show", f"{ref}:{path.as_posix()}"], text=True)
    )


def rows_for_fixture(fixture: dict[str, Any], branch: str) -> list[dict[str, Any]]:
    rows = []
    for field, measure in TIMECOURSES.items():
        for point in fixture[field]:
            for group, value in enumerate(point["grouped_values"]):
                rows.append(
                    {
                        "scenario": fixture["scenario"],
                        "measure": measure,
                        "panel": f"{measure} - Group {group + 1}",
                        "group": f"Group {group + 1}",
                        "time": point["time"],
                        "branch": branch,
                        "value": value,
                    }
                )
    return rows


def make_chart(rows: list[dict[str, Any]]) -> alt.Chart:
    chart = (
        alt.Chart(alt.Data(values=rows))
        .mark_line()
        .encode(
            x=alt.X("time:Q", title="Time (days)"),
            y=alt.Y("value:Q", title="Value"),
            color=alt.Color("branch:N", title="Version"),
            tooltip=[
                alt.Tooltip("scenario:N", title="Scenario"),
                alt.Tooltip("measure:N", title="Measure"),
                alt.Tooltip("group:N", title="Group"),
                alt.Tooltip("time:Q", title="Day"),
                alt.Tooltip("branch:N", title="Version"),
                alt.Tooltip("value:Q", title="Value", format=",.3f"),
            ],
        )
    )
    return chart.properties(width=220, height=140).facet(
        row=alt.Row("panel:N", title=None),
        column=alt.Column("scenario:N", title=None),
    )


def print_summary(current: dict[str, Any], base: dict[str, Any], base_ref: str) -> None:
    print(f"\nSummary versus {base_ref}: {current['scenario']}")
    print("measure / group                         max |delta|    day    cumulative (main -> current)    cumulative delta")
    for field, measure in TIMECOURSES.items():
        current_series = current[field]
        base_series = base[field]
        for group in range(len(current_series[0]["grouped_values"])):
            comparisons = [
                (
                    abs(current_point["grouped_values"][group] - base_point["grouped_values"][group]),
                    current_point["time"],
                )
                for current_point, base_point in zip(current_series, base_series)
            ]
            max_delta, max_time = max(comparisons)
            current_total = sum(point["grouped_values"][group] for point in current_series)
            base_total = sum(point["grouped_values"][group] for point in base_series)
            cumulative_delta = current_total - base_total
            cumulative_percent = 100 * cumulative_delta / base_total if base_total else float("nan")
            label = f"{measure} / Group {group + 1}"
            print(
                f"{label:<38} {max_delta:>12,.3f} {max_time:>6.0f}"
                f" {base_total:>15,.3f} -> {current_total:<15,.3f}"
                f" {cumulative_delta:>+15,.3f} ({cumulative_percent:+.4f}%)"
            )

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Visualize working-tree snapshot timecourses against a git ref."
    )
    parser.add_argument("--base-ref", default="main", help="Git ref to compare against")
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("snapshot-timecourse-comparison.html"),
        help="Output HTML path",
    )
    args = parser.parse_args()

    root = git_root()
    snapshot_dir = root / "model" / "tests" / "snapshot-data"
    rows = []
    for name in fixture_names(snapshot_dir):
        path = Path("model") / "tests" / "snapshot-data" / f"{name}.json"
        current = load_fixture(snapshot_dir / path.name)
        base = load_fixture(path, args.base_ref)
        rows.extend(rows_for_fixture(current, "working tree"))
        rows.extend(rows_for_fixture(base, args.base_ref))
        print_summary(current, base, args.base_ref)

    alt.data_transformers.disable_max_rows()
    chart = make_chart(rows).resolve_scale(y="independent")
    output = args.output if args.output.is_absolute() else Path.cwd() / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    chart.save(output)
    print(f"wrote {output} ({len(rows):,} plotted points)")


if __name__ == "__main__":
    main()
