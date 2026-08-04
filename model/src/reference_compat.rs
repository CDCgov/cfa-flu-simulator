// Equivalence test against the upstream dynode-web reference model.
//
// Fixtures under `model/tests/reference-data/` are produced by
// `scripts/gen_reference/` running the upstream model. Each fixture pins
// a single mitigation scenario (no_mitigations / vaccine_only /
// antivirals_only / community_only / ttiq_only) at the shared default
// parameter set. This test runs the local model with the same parameters
// and asserts the time-series outputs match within a tight tolerance.

#![cfg(test)]

use crate::model::SEIRModel;
use crate::model_unified::{DynodeModel, OutputItemGrouped, OutputType};
use crate::parameters::{Parameters, ParametersTyped};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct RefItem {
    time: f64,
    grouped_values: Vec<f64>,
}

#[derive(Debug, Deserialize)]
struct Fixture {
    #[allow(dead_code)]
    scenario: String,
    days: usize,
    infection_incidence: Vec<RefItem>,
    symptomatic_incidence: Vec<RefItem>,
    hospital_incidence: Vec<RefItem>,
    death_incidence: Vec<RefItem>,
}

fn load_fixture(name: &str) -> Fixture {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/reference-data")
        .join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&raw).expect("fixture parses")
}

// Build params matching the reference's Parameters::<2>::default() with
// exactly one mitigation enabled (or none for "no_mitigations"). The TOML
// defaults already line up with the reference except for p_test_sympto
// (0.001 vs 0.0) and vaccine_enabled (true vs false), so we override those.
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

fn assert_series_close(actual: &[OutputItemGrouped], expected: &[RefItem], label: &str) {
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
fn reference_no_mitigations() {
    check_scenario("no_mitigations");
}

#[test]
fn reference_vaccine_only() {
    check_scenario("vaccine_only");
}

#[test]
fn reference_antivirals_only() {
    check_scenario("antivirals_only");
}

#[test]
fn reference_community_only() {
    check_scenario("community_only");
}

#[test]
fn reference_ttiq_only() {
    check_scenario("ttiq_only");
}
