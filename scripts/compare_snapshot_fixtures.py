#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = [
#   "altair>=5.0",
#   "vl-convert-python>=1.0",
# ]
# ///
"""Compare freshly generated Rust model snapshots with committed fixtures.

The script emits one large SVG containing four panels for each fixture
scenario:

* summary totals, current versus committed;
* summary differences (current minus committed);
* total infection incidence over time, current versus committed; and
* time-varying infection-incidence differences (current minus committed).

Run from the repository root, or pass ``--repo-root`` explicitly::

    uv run scripts/compare_snapshot_fixtures.py
    uv run scripts/compare_snapshot_fixtures.py --output /tmp/snapshots.svg
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

import altair as alt

SCENARIOS = (
    "no_mitigations",
    "vaccine_only",
    "antivirals_only",
    "community_only",
    "ttiq_only",
)


def generate_current_fixtures(repo_root: Path) -> dict[str, dict[str, Any]]:
    """Run the fixture generator and parse its aggregate JSON from stdout."""
    result = subprocess.run(
        ["cargo", "run", "--quiet", "--bin", "update_snapshot", "--", "--stdout"],
        cwd=repo_root / "model",
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def load_committed_fixtures(repo_root: Path) -> dict[str, dict[str, Any]]:
    path = repo_root / "model" / "tests" / "snapshot-data" / "snapshots.json"
    with path.open(encoding="utf-8") as file:
        fixtures = json.load(file)
    return fixtures


def total_series(fixture: dict[str, Any], field: str) -> list[dict[str, float]]:
    """Collapse the fixture's grouped incidence values to one total per time."""
    return [
        {
            "time": float(item["time"]),
            "value": float(sum(item["grouped_values"])),
        }
        for item in fixture[field]
    ]


def summary_rows(
    scenario: str, current: dict[str, Any], committed: dict[str, Any]
) -> list[dict[str, Any]]:
    rows = []
    for field in [
        "total_infections",
        "total_symptomatic_infections",
        "total_hospitalizations",
        "total_deaths",
    ]:
        current_value = float(current["summary"][field])
        committed_value = float(committed["summary"][field])
        rows.extend(
            [
                {
                    "scenario": scenario,
                    "metric": field,
                    "series": "Current",
                    "value": current_value,
                },
                {
                    "scenario": scenario,
                    "metric": field,
                    "series": "Committed",
                    "value": committed_value,
                },
            ]
        )
    return rows


def summary_difference_rows(
    scenario: str, current: dict[str, Any], committed: dict[str, Any]
) -> list[dict[str, Any]]:
    return [
        {
            "scenario": scenario,
            "metric": field,
            "difference": float(current["summary"][field])
            - float(committed["summary"][field]),
        }
        for field in [
            "total_infections",
            "total_symptomatic_infections",
            "total_hospitalizations",
            "total_deaths",
        ]
    ]


def timecourse_rows(
    scenario: str, current: dict[str, Any], committed: dict[str, Any]
) -> list[dict[str, Any]]:
    rows = []
    for series_name, fixture in (("Current", current), ("Committed", committed)):
        for point in total_series(fixture, "infection_incidence"):
            rows.append(
                {
                    "scenario": scenario,
                    "metric": "Infections",
                    "series": series_name,
                    "time": point["time"],
                    "value": point["value"],
                }
            )
    return rows


def timecourse_difference_rows(
    scenario: str, current: dict[str, Any], committed: dict[str, Any]
) -> list[dict[str, Any]]:
    rows = []
    current_points = total_series(current, "infection_incidence")
    committed_points = total_series(committed, "infection_incidence")
    if len(current_points) != len(committed_points):
        raise ValueError(
            f"{scenario}: infection_incidence has different time-grid lengths"
        )
    for current_point, committed_point in zip(current_points, committed_points):
        if current_point["time"] != committed_point["time"]:
            raise ValueError(
                f"{scenario}: infection_incidence has different time grids"
            )
        rows.append(
            {
                "scenario": scenario,
                "metric": "Infections",
                "time": current_point["time"],
                "difference": current_point["value"] - committed_point["value"],
            }
        )
    return rows


def comparison_statistics(
    scenario: str, current: dict[str, Any], committed: dict[str, Any]
) -> dict[str, Any]:
    summary_differences = [
        (
            field,
            float(current["summary"][field]) - float(committed["summary"][field]),
        )
        for field in [
            "total_infections",
            "total_symptomatic_infections",
            "total_hospitalizations",
            "total_deaths",
        ]
    ]
    summary_metric, summary_difference = max(
        summary_differences, key=lambda item: abs(item[1])
    )
    infection_differences = timecourse_difference_rows(scenario, current, committed)
    largest_infection = max(
        infection_differences, key=lambda item: abs(item["difference"])
    )
    return {
        "scenario": scenario,
        "summary_metric": summary_metric,
        "summary_difference": summary_difference,
        "infection_difference": largest_infection["difference"],
        "infection_time": largest_infection["time"],
        "infection_nonzero_points": sum(
            point["difference"] != 0.0 for point in infection_differences
        ),
        "infection_points": len(infection_differences),
    }


def print_summary_statistics(
    current_fixtures: dict[str, dict[str, Any]],
    committed_fixtures: dict[str, dict[str, Any]],
) -> None:
    statistics = [
        comparison_statistics(
            scenario, current_fixtures[scenario], committed_fixtures[scenario]
        )
        for scenario in SCENARIOS
    ]
    largest_summary = max(statistics, key=lambda item: abs(item["summary_difference"]))
    largest_infection = max(
        statistics, key=lambda item: abs(item["infection_difference"])
    )

    print("Comparison summary")
    print("==================")
    print()

    for item in statistics:
        print(f"{item['scenario']}")
        print(
            f"  max |summary difference|: {abs(item['summary_difference']):.6g} "
            f"({item['summary_metric']})"
        )
        print(
            f"  max |infection difference|: {abs(item['infection_difference']):.6g} "
            f"(day {item['infection_time']:.2f})"
        )
        print(
            f"  differing infection points: "
            f"{item['infection_nonzero_points']}/{item['infection_points']}"
        )
        print()

    print(
        f"Overall max |summary Δ|: {abs(largest_summary['summary_difference']):.6g} "
        f"({largest_summary['scenario']}, {largest_summary['summary_metric']})"
    )
    print(
        f"Overall max |infection Δ|: {abs(largest_infection['infection_difference']):.6g} "
        f"({largest_infection['scenario']}, day {largest_infection['infection_time']:.2f})"
    )


def make_summary_chart(
    scenario: str, current: dict[str, Any], committed: dict[str, Any]
) -> alt.Chart:
    data = summary_rows(scenario, current, committed)
    return (
        alt.Chart(alt.Data(values=data))
        .mark_bar()
        .encode(
            x=alt.X(
                "metric:N",
                title=None,
                sort=[
                    "total_infections",
                    "total_symptomatic_infections",
                    "total_hospitalizations",
                    "total_deaths",
                ],
            ),
            xOffset=alt.XOffset("series:N", title=None),
            y=alt.Y("value:Q", title="Count"),
            color=alt.Color("series:N", title=None),
        )
        .properties(title=f"{scenario} — summary values", width=300, height=220)
    )


def make_summary_difference_chart(
    scenario: str, current: dict[str, Any], committed: dict[str, Any]
) -> alt.Chart:
    data = summary_difference_rows(scenario, current, committed)
    return (
        alt.Chart(alt.Data(values=data))
        .mark_bar()
        .encode(
            x=alt.X(
                "metric:N",
                title=None,
                sort=[
                    "total_infections",
                    "total_symptomatic_infections",
                    "total_hospitalizations",
                    "total_deaths",
                ],
            ),
            y=alt.Y("difference:Q", title="Current − committed"),
        )
        .properties(title=f"{scenario} — summary differences", width=300, height=220)
    )


def make_timecourse_chart(
    scenario: str, current: dict[str, Any], committed: dict[str, Any]
) -> alt.Chart:
    data = timecourse_rows(scenario, current, committed)
    return (
        alt.Chart(alt.Data(values=data))
        .mark_line()
        .encode(
            x=alt.X("time:Q", title="Time (days)"),
            y=alt.Y("value:Q", title="Total infections"),
            color=alt.Color("series:N", title=None),
        )
        .properties(title=f"{scenario} — infection timecourse", width=300, height=220)
    )


def make_timecourse_difference_chart(
    scenario: str, current: dict[str, Any], committed: dict[str, Any]
) -> alt.Chart:
    data = timecourse_difference_rows(scenario, current, committed)
    return (
        alt.Chart(alt.Data(values=data))
        .mark_line(color="#9467bd")
        .encode(
            x=alt.X("time:Q", title="Time (days)"),
            y=alt.Y("difference:Q", title="Current - committed"),
        )
        .properties(
            title=f"{scenario} — time-varying infection difference",
            width=300,
            height=220,
        )
    )


def build_chart(
    current_fixtures: dict[str, dict[str, Any]],
    committed_fixtures: dict[str, dict[str, Any]],
) -> alt.VConcatChart:
    rows = []
    for scenario in SCENARIOS:
        committed = committed_fixtures[scenario]
        current = current_fixtures[scenario]
        rows.append(
            alt.hconcat(
                make_summary_chart(scenario, current, committed),
                make_summary_difference_chart(scenario, current, committed),
                make_timecourse_chart(scenario, current, committed),
                make_timecourse_difference_chart(scenario, current, committed),
            )
        )
    return alt.vconcat(*rows).properties(
        title="Current model output versus committed snapshot fixtures"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="repository root (default: inferred from this script)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="output SVG path (default: next to the committed fixture)",
    )
    args = parser.parse_args()
    repo_root = args.repo_root.resolve()
    output = (
        args.output.resolve()
        if args.output is not None
        else repo_root / "model" / "tests" / "snapshot-data" / "snapshot-comparison.svg"
    )

    current_fixtures = generate_current_fixtures(repo_root)
    committed_fixtures = load_committed_fixtures(repo_root)
    chart = build_chart(current_fixtures, committed_fixtures)
    output.parent.mkdir(parents=True, exist_ok=True)
    chart.save(output)
    print_summary_statistics(current_fixtures, committed_fixtures)
    print(f"\nwrote {output}")


if __name__ == "__main__":
    main()
