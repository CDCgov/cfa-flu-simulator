// Snapshot tests for established model behavior.
//
// Each fixture under `model/tests/snapshot-data/` captures the output of one
// mitigation scenario at the shared default parameter set. These tests guard
// against unintended changes to the existing time series.

#![cfg(test)]

use crate::model::SEIRModel;
use crate::model_unified::{EpidemicModel, OutputItemGrouped, OutputType};
use crate::parameters::{Parameters, ParametersTyped};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct SnapshotItem {
    time: f64,
    grouped_values: Vec<f64>,
}

#[derive(Debug, Deserialize)]
struct Fixture {
    #[allow(dead_code)]
    scenario: String,
    days: usize,
    infection_incidence: Vec<SnapshotItem>,
    symptomatic_incidence: Vec<SnapshotItem>,
    hospital_incidence: Vec<SnapshotItem>,
    death_incidence: Vec<SnapshotItem>,
}

fn load_fixture(name: &str) -> Fixture {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshot-data")
        .join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&raw).expect("fixture parses")
}

// Build the parameter scenarios captured by the stored snapshots. Each has
// exactly one mitigation enabled, or none for `no_mitigations`.
fn make_params(scenario: &str) -> Parameters {
    let mut p = Parameters::default();
    p.p_test_sympto = 0.0;
    p.vaccine_enabled = false;
    p.antivirals_enabled = false;
    p.community_enabled = false;
    p.ttiq_enabled = false;
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

fn assert_series_close(actual: &[OutputItemGrouped], expected: &[SnapshotItem], label: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{label}: time-grid length differs (actual {} vs expected {})",
        actual.len(),
        expected.len()
    );
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (a.time - e.time).abs() < 1e-9,
            "{label}: time[{i}] differs: actual {} vs expected {}",
            a.time,
            e.time,
        );
        assert_eq!(
            a.grouped_values.len(),
            e.grouped_values.len(),
            "{label}: row[{i}] (t={}) group count differs",
            a.time,
        );
        // Hybrid tol: 1e-6 absolute floor (handles near-zero values) plus
        // 1e-9 relative for large values (peak incidence is ~1e6 at
        // population 330M, where ULP-level reordering across machines could
        // exceed pure abs tol).
        for (j, (av, ev)) in a.grouped_values.iter().zip(&e.grouped_values).enumerate() {
            let tol = 1e-6_f64.max(1e-9 * ev.abs());
            assert!(
                (av - ev).abs() <= tol,
                "{label}: t={} group={j} differs: actual {av} vs expected {ev} (tol {tol})",
                a.time,
            );
        }
    }
}

fn check_scenario(scenario: &str) {
    let expected = load_fixture(scenario);
    let typed: ParametersTyped<2> = make_params(scenario).try_into().expect("params -> typed");
    let actual = SEIRModel::new(typed).integrate(expected.days);

    let pairs = [
        (
            OutputType::InfectionIncidence,
            &expected.infection_incidence,
            "infection_incidence",
        ),
        (
            OutputType::SymptomaticIncidence,
            &expected.symptomatic_incidence,
            "symptomatic_incidence",
        ),
        (
            OutputType::HospitalIncidence,
            &expected.hospital_incidence,
            "hospital_incidence",
        ),
        (
            OutputType::DeathIncidence,
            &expected.death_incidence,
            "death_incidence",
        ),
    ];
    for (output_type, expected_series, name) in pairs {
        assert_series_close(
            actual.get_output(&output_type),
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
