// Regression snapshots for the native Rust model.
//
// Fixtures under `model/tests/snapshot-data/` are generated from this crate's
// current implementation by running the ignored `update_snapshots` test. Normal
// tests only read committed fixtures and compare with a tolerant numeric check.

#![cfg(test)]

use crate::model::SEIRModel;
use crate::model_unified::{DynodeModel, OutputItemGrouped, OutputType};
use crate::parameters::{Parameters, ParametersTyped};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DAYS: usize = 200;
const ABSOLUTE_TOLERANCE: f64 = 1e-6;
const RELATIVE_TOLERANCE: f64 = 1e-9;
const SCENARIOS: [&str; 5] = [
    "no_mitigations",
    "vaccine_only",
    "antivirals_only",
    "community_only",
    "ttiq_only",
];

#[derive(Debug, Serialize, Deserialize)]
struct FixtureMetadata {
    model_crate_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Summary {
    total_infections: f64,
    total_symptomatic_infections: f64,
    total_hospitalizations: f64,
    total_deaths: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Fixture {
    metadata: FixtureMetadata,
    scenario: String,
    summary: Summary,
    infection_incidence: Vec<OutputItemGrouped>,
    symptomatic_incidence: Vec<OutputItemGrouped>,
    hospital_incidence: Vec<OutputItemGrouped>,
    death_incidence: Vec<OutputItemGrouped>,
}

fn snapshot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshot-data")
}

fn load_fixture(name: &str) -> Fixture {
    let path = snapshot_dir().join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&raw).expect("fixture parses")
}

fn make_params(scenario: &str) -> Parameters {
    let mut p = Parameters {
        p_test_sympto: 0.0,
        vaccine_enabled: false,
        antivirals_enabled: false,
        community_enabled: false,
        ttiq_enabled: false,
        ..Default::default()
    };
    match scenario {
        "no_mitigations" => {}
        "vaccine_only" => p.vaccine_enabled = true,
        "antivirals_only" => p.antivirals_enabled = true,
        "community_only" => p.community_enabled = true,
        "ttiq_only" => p.ttiq_enabled = true,
        other => panic!("unknown scenario {other}"),
    }
    p
}

fn sum_series(series: &[OutputItemGrouped]) -> f64 {
    series.iter().flat_map(|item| &item.grouped_values).sum()
}

fn summarize(fixture: &Fixture) -> Summary {
    Summary {
        total_infections: sum_series(&fixture.infection_incidence),
        total_symptomatic_infections: sum_series(&fixture.symptomatic_incidence),
        total_hospitalizations: sum_series(&fixture.hospital_incidence),
        total_deaths: sum_series(&fixture.death_incidence),
    }
}

fn run_scenario(scenario: &str) -> Fixture {
    let typed: ParametersTyped<2> = make_params(scenario).try_into().expect("params -> typed");
    let actual = SEIRModel::new(typed).integrate(DAYS);

    let mut fixture = Fixture {
        metadata: FixtureMetadata {
            model_crate_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        scenario: scenario.to_string(),
        summary: Summary {
            total_infections: 0.0,
            total_symptomatic_infections: 0.0,
            total_hospitalizations: 0.0,
            total_deaths: 0.0,
        },
        infection_incidence: actual.get_output(&OutputType::InfectionIncidence).clone(),
        symptomatic_incidence: actual.get_output(&OutputType::SymptomaticIncidence).clone(),
        hospital_incidence: actual.get_output(&OutputType::HospitalIncidence).clone(),
        death_incidence: actual.get_output(&OutputType::DeathIncidence).clone(),
    };
    fixture.summary = summarize(&fixture);
    fixture
}

fn assert_value_close(actual: f64, expected: f64, label: &str) {
    let tol = ABSOLUTE_TOLERANCE.max(RELATIVE_TOLERANCE * expected.abs());
    assert!(
        (actual - expected).abs() <= tol,
        "{label} differs: actual {actual} vs expected {expected} (tol {tol})",
    );
}

fn assert_series_close(actual: &[OutputItemGrouped], expected: &[OutputItemGrouped], label: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{label}: time-grid length differs (actual {} vs expected {})",
        actual.len(),
        expected.len()
    );
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
        assert_value_close(a.time, e.time, &format!("{label}: time[{i}]"));
        assert_eq!(
            a.grouped_values.len(),
            e.grouped_values.len(),
            "{label}: row[{i}] (t={}) group count differs",
            a.time,
        );
        for (j, (av, ev)) in a.grouped_values.iter().zip(&e.grouped_values).enumerate() {
            assert_value_close(*av, *ev, &format!("{label}: t={} group={j}", a.time));
        }
    }
}

fn assert_summary_close(actual: &Summary, expected: &Summary, scenario: &str) {
    assert_value_close(
        actual.total_infections,
        expected.total_infections,
        &format!("{scenario} total_infections"),
    );
    assert_value_close(
        actual.total_symptomatic_infections,
        expected.total_symptomatic_infections,
        &format!("{scenario} total_symptomatic_infections"),
    );
    assert_value_close(
        actual.total_hospitalizations,
        expected.total_hospitalizations,
        &format!("{scenario} total_hospitalizations"),
    );
    assert_value_close(
        actual.total_deaths,
        expected.total_deaths,
        &format!("{scenario} total_deaths"),
    );
}

fn check_scenario(scenario: &str) {
    let expected = load_fixture(scenario);
    let actual = run_scenario(scenario);

    assert_eq!(expected.scenario, scenario);
    assert_summary_close(&actual.summary, &expected.summary, scenario);

    let pairs = [
        (
            &actual.infection_incidence,
            &expected.infection_incidence,
            "infection_incidence",
        ),
        (
            &actual.symptomatic_incidence,
            &expected.symptomatic_incidence,
            "symptomatic_incidence",
        ),
        (
            &actual.hospital_incidence,
            &expected.hospital_incidence,
            "hospital_incidence",
        ),
        (
            &actual.death_incidence,
            &expected.death_incidence,
            "death_incidence",
        ),
    ];
    for (actual_series, expected_series, name) in pairs {
        assert_series_close(
            actual_series,
            expected_series,
            &format!("{scenario} {name}"),
        );
    }
}

#[test]
fn snapshot_no_mitigations() {
    check_scenario("no_mitigations");
}

#[test]
fn snapshot_vaccine_only() {
    check_scenario("vaccine_only");
}

#[test]
fn snapshot_antivirals_only() {
    check_scenario("antivirals_only");
}

#[test]
fn snapshot_community_only() {
    check_scenario("community_only");
}

#[test]
fn snapshot_ttiq_only() {
    check_scenario("ttiq_only");
}

#[test]
#[ignore = "regenerates committed model snapshot fixtures"]
fn update_snapshots() {
    let out_dir = snapshot_dir();
    std::fs::create_dir_all(&out_dir).expect("create snapshot-data dir");

    for scenario in SCENARIOS {
        let fixture = run_scenario(scenario);
        let path = out_dir.join(format!("{scenario}.json"));
        let json = serde_json::to_string_pretty(&fixture).expect("serialize fixture");
        std::fs::write(&path, format!("{json}\n")).expect("write fixture");
        println!("wrote {}", path.display());
    }
}
