#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = [
#   "altair>=5.0",
#   "vl-convert-python>=1.0",
# ]
# ///
"""Compare freshly generated Rust model snapshot with committed fixture.

The script emits one large SVG containing four panels for each fixture
scenario:

* summary totals, current versus committed;
* summary differences (current minus committed);
* total infection incidence over time, current versus committed; and
* time-varying infection-incidence differences (current minus committed).

Run from the repository root, or pass --repo-root explicitly:

    uv run scripts/compare_snapshot_fixtures.py
    uv run scripts/compare_snapshot_fixtures.py --output /tmp/snapshots.svg
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any, Self

import altair as alt  # ty: ignore[unresolved-import]


class FixtureScenario:
    def __init__(self, content: dict[str, Any]):
        self.content = content

    def total_series(self, field: str) -> list[dict[str, float]]:
        """Collapse grouped incidence values to one total per time."""
        return [
            {
                "time": float(item["time"]),
                "value": float(sum(item["grouped_values"])),
            }
            for item in self.content[field]
        ]

    def summary_value(self, field: str) -> float:
        return float(self.content["summary"][field])


class Fixture:
    def __init__(self, content: dict[str, dict[str, Any]]):
        self.scenarios = {
            name: FixtureScenario(scenario) for name, scenario in content.items()
        }

    def __getitem__(self, scenario: str) -> FixtureScenario:
        return self.scenarios[scenario]

    @property
    def names(self) -> list[str]:
        return sorted(self.scenarios.keys())

    @classmethod
    def from_current(cls, model_dir: Path) -> Self:
        result = subprocess.run(
            ["cargo", "run", "--quiet", "--bin", "update_snapshot", "--", "--stdout"],
            cwd=model_dir,
            check=True,
            stdout=subprocess.PIPE,
            text=True,
        )
        content = json.loads(result.stdout)
        return cls(content)

    @classmethod
    def from_file(cls, path: Path) -> Self:
        with path.open(encoding="utf-8") as file:
            content = json.load(file)
        return cls(content)


class Comparison:
    def __init__(self, current: Fixture, committed: Fixture):
        assert current.names == committed.names, (
            "Current and committed fixtures have different scenario names"
        )

        self.current = current
        self.committed = committed
        self.names = current.names

    def scenarios(self, name: str) -> tuple[FixtureScenario, FixtureScenario]:
        return self.current.scenarios[name], self.committed.scenarios[name]

    def summary_rows(self, scenario: str) -> list[dict[str, Any]]:
        rows = []
        for field in [
            "total_infections",
            "total_symptomatic_infections",
            "total_hospitalizations",
            "total_deaths",
        ]:
            current_value = self.current[scenario].summary_value(field)
            committed_value = self.committed[scenario].summary_value(field)
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

    def summary_difference_rows(self, scenario: str) -> list[dict[str, Any]]:
        return [
            {
                "scenario": scenario,
                "metric": field,
                "difference": self.current[scenario].summary_value(field)
                - self.committed[scenario].summary_value(field),
            }
            for field in [
                "total_infections",
                "total_symptomatic_infections",
                "total_hospitalizations",
                "total_deaths",
            ]
        ]

    def timecourse_rows(self, scenario: str) -> list[dict[str, Any]]:
        rows = []
        for series_name, fixture in (
            ("Current", self.current[scenario]),
            ("Committed", self.committed[scenario]),
        ):
            for point in fixture.total_series("infection_incidence"):
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

    def timecourse_difference_rows(self, scenario: str) -> list[dict[str, Any]]:
        rows = []
        current_points = self.current[scenario].total_series("infection_incidence")
        committed_points = self.committed[scenario].total_series("infection_incidence")
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

    def comparison_statistics(self, scenario: str) -> dict[str, Any]:
        summary_differences = [
            (
                field,
                self.current[scenario].summary_value(field)
                - self.committed[scenario].summary_value(field),
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
        infection_differences = self.timecourse_difference_rows(scenario)
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

    def print_summary_statistics(self) -> None:
        statistics = [self.comparison_statistics(scenario) for scenario in self.names]
        largest_summary = max(
            statistics, key=lambda item: abs(item["summary_difference"])
        )
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

    def make_summary_chart(self, scenario: str) -> alt.Chart:
        data = self.summary_rows(scenario)
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

    def make_summary_difference_chart(self, scenario: str) -> alt.Chart:
        data = self.summary_difference_rows(scenario)
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
            .properties(
                title=f"{scenario} — summary differences", width=300, height=220
            )
        )

    def make_timecourse_chart(self, scenario: str) -> alt.Chart:
        data = self.timecourse_rows(scenario)
        return (
            alt.Chart(alt.Data(values=data))
            .mark_line()
            .encode(
                x=alt.X("time:Q", title="Time (days)"),
                y=alt.Y("value:Q", title="Total infections"),
                color=alt.Color("series:N", title=None),
            )
            .properties(
                title=f"{scenario} — infection timecourse", width=300, height=220
            )
        )

    def make_timecourse_difference_chart(self, scenario: str) -> alt.Chart:
        data = self.timecourse_difference_rows(scenario)
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

    def build_chart(self) -> alt.VConcatChart:
        rows = [
            alt.hconcat(
                self.make_summary_chart(scenario),
                self.make_summary_difference_chart(scenario),
                self.make_timecourse_chart(scenario),
                self.make_timecourse_difference_chart(scenario),
            )
            for scenario in self.names
        ]

        chart = alt.vconcat(*rows).properties(
            title="Current model output versus committed snapshot fixtures"
        )

        return chart


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

    model_dir = repo_root / "model"
    fixture_path = repo_root / "model" / "tests" / "snapshot-data" / "snapshots.json"

    comparison = Comparison(
        Fixture.from_current(model_dir=model_dir), Fixture.from_file(path=fixture_path)
    )

    comparison.print_summary_statistics()

    chart = comparison.build_chart()
    output.parent.mkdir(parents=True, exist_ok=True)
    chart.save(output)
    print(f"\nwrote {output}")


if __name__ == "__main__":
    main()
