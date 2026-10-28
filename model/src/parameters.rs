use nalgebra::{SMatrix, SVector};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::mitigations::{
    AntiviralsParams, CommunityMitigationParamsTyped, MitigationParamsTyped, VaccineParams,
};

// Public, flat, TOML/JSON/wasm-serializable parameter struct.
// Mitigation fields are flattened with prefixes (vaccine_, antivirals_,
// community_) so the full schema can round-trip through URL query
// strings and flat forms. Solver code works on `ParametersTyped<N>`, which
// keeps the mitigation grouping for readability.
#[derive(Tsify, Debug, Clone, Serialize, Deserialize)]
pub struct Parameters {
    pub n: usize,
    // Integration horizon passed to `SEIRModelUnified::run`. Not part of
    // the solver state but belongs with the inputs that drive a run.
    pub days: usize,
    pub population: f64,
    pub population_fraction_labels: Vec<String>,
    pub population_fractions: Vec<f64>,
    // n*n matrix
    // Values are stored column-major to match nalgebra's native layout.
    pub contact_matrix: Vec<f64>,
    pub initial_infections: f64,
    pub fraction_initial_immune: f64,
    pub r0: f64,
    pub latent_period: f64,
    pub infectious_period: f64,
    pub p_symp_given_inf: Vec<f64>,
    pub p_hosp_given_symp: Vec<f64>,
    pub inf_hosp_delay: f64,
    pub p_death_given_hosp: Vec<f64>,
    pub inf_death_delay: f64,

    pub vaccine_enabled: bool,
    pub vaccine_doses: usize,
    pub vaccine_start: f64,
    pub vaccine_dose2_delay: f64,
    pub vaccine_p_get_2_doses: f64,
    pub vaccine_administration_rate: f64,
    pub vaccine_doses_available: f64,
    pub vaccine_ramp_up: f64,
    pub vaccine_ve_s: f64,
    pub vaccine_ve_i: f64,
    pub vaccine_ve_p: f64,
    pub vaccine_ve_2s: f64,
    pub vaccine_ve_2i: f64,
    pub vaccine_ve_2p: f64,

    pub antivirals_enabled: bool,
    pub antivirals_fraction_outpatient: f64,
    pub antivirals_fraction_inpatient: f64,
    pub antivirals_ave_i: f64,
    pub antivirals_ave_p_hosp: f64,
    pub antivirals_ave_p_death: f64,

    pub community_enabled: bool,
    pub community_start: f64,
    pub community_duration: f64,
    pub community_effectiveness: Vec<f64>,
}

impl Parameters {
    pub fn has_mitigations(&self) -> bool {
        self.vaccine_enabled || self.antivirals_enabled || self.community_enabled
    }

    pub fn without_mitigations(&self) -> Self {
        let mut params = self.clone();
        params.vaccine_enabled = false;
        params.antivirals_enabled = false;
        params.community_enabled = false;
        params
    }

    // Validate that all n-sized fields match `n`. Called once at construction.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.population_fractions.len() != self.n {
            return Err("population_fractions length != n");
        }
        if self.population_fraction_labels.len() != self.n {
            return Err("population_fraction_labels length != n");
        }
        if self.p_symp_given_inf.len() != self.n {
            return Err("p_symp_given_inf  length != n");
        }
        if self.p_hosp_given_symp.len() != self.n {
            return Err("p_hosp_given_symp length != n");
        }
        if self.p_death_given_hosp.len() != self.n {
            return Err("p_death_given_hosp length != n");
        }
        if self.contact_matrix.len() != self.n * self.n {
            return Err("contact_matrix length != n*n");
        }
        if self.community_effectiveness.len() != self.n * self.n {
            return Err("community effectiveness length != n*n");
        }
        Ok(())
    }
}

pub(crate) fn test_default_parameters() -> Parameters {
    toml::from_str(include_str!("../tests/fixtures/default-params.toml"))
        .expect("test default parameters are malformed")
}

#[cfg(test)]
impl Default for Parameters {
    fn default() -> Self {
        test_default_parameters()
    }
}

// Solver-internal typed counterpart.
#[derive(Debug, Clone)]
pub(crate) struct ParametersTyped<const N: usize> {
    pub days: usize,
    pub population: f64,
    pub population_fractions: SVector<f64, N>,
    pub population_fraction_labels: SVector<String, N>,
    pub contact_matrix: SMatrix<f64, N, N>,
    pub initial_infections: f64,
    pub fraction_initial_immune: f64,
    pub r0: f64,
    pub latent_period: f64,
    pub infectious_period: f64,
    // probability symptomatic given infection; i.e., fraction symptomatic (FS)
    pub p_symp_given_inf: SVector<f64, N>,
    // prob. hospitalized given symptomatic; i.e., symptomatic-hospitalization ratio (SHR)
    pub p_hosp_given_symp: SVector<f64, N>,
    // delay from infection to hospitalization
    pub inf_hosp_delay: f64,
    // prob. death given hospitalization; P[D|H] = CFR / SHR
    pub p_death_given_hosp: SVector<f64, N>,
    pub inf_death_delay: f64,
    pub mitigations: MitigationParamsTyped<N>,
}

impl<const N: usize> ParametersTyped<N> {
    #[allow(dead_code)]
    pub fn has_mitigations(&self) -> bool {
        self.mitigations.antivirals.enabled
            || self.mitigations.community.enabled
            || self.mitigations.vaccine.enabled
    }

    #[allow(dead_code)]
    pub fn without_mitigations(&self) -> Self {
        let mut params = self.clone();
        params.mitigations.antivirals.enabled = false;
        params.mitigations.community.enabled = false;
        params.mitigations.vaccine.enabled = false;
        params
    }
}

impl<const N: usize> TryFrom<Parameters> for ParametersTyped<N> {
    type Error = &'static str;
    fn try_from(params: Parameters) -> Result<Self, Self::Error> {
        if params.n != N {
            return Err("Parameters.n does not match target ParametersTyped<N>");
        }
        params.validate()?;
        let vaccine = VaccineParams {
            enabled: params.vaccine_enabled,
            doses: params.vaccine_doses,
            start: params.vaccine_start,
            dose2_delay: params.vaccine_dose2_delay,
            p_get_2_doses: params.vaccine_p_get_2_doses,
            administration_rate: params.vaccine_administration_rate,
            doses_available: params.vaccine_doses_available,
            ramp_up: params.vaccine_ramp_up,
            ve_s: params.vaccine_ve_s,
            ve_i: params.vaccine_ve_i,
            ve_p: params.vaccine_ve_p,
            ve_2s: params.vaccine_ve_2s,
            ve_2i: params.vaccine_ve_2i,
            ve_2p: params.vaccine_ve_2p,
        };
        let antivirals = AntiviralsParams {
            enabled: params.antivirals_enabled,
            fraction_outpatient: params.antivirals_fraction_outpatient,
            fraction_inpatient: params.antivirals_fraction_inpatient,
            ave_i: params.antivirals_ave_i,
            ave_p_hosp: params.antivirals_ave_p_hosp,
            ave_p_death: params.antivirals_ave_p_death,
        };
        let community = CommunityMitigationParamsTyped {
            enabled: params.community_enabled,
            start: params.community_start,
            duration: params.community_duration,
            effectiveness: SMatrix::from_iterator(params.community_effectiveness),
        };
        Ok(ParametersTyped {
            days: params.days,
            population: params.population,
            population_fractions: SVector::from_iterator(params.population_fractions),
            population_fraction_labels: SVector::from_iterator(params.population_fraction_labels),
            contact_matrix: SMatrix::from_iterator(params.contact_matrix),
            initial_infections: params.initial_infections,
            fraction_initial_immune: params.fraction_initial_immune,
            r0: params.r0,
            latent_period: params.latent_period,
            infectious_period: params.infectious_period,
            p_symp_given_inf: SVector::from_iterator(params.p_symp_given_inf),
            p_hosp_given_symp: SVector::from_iterator(params.p_hosp_given_symp),
            inf_hosp_delay: params.inf_hosp_delay,
            p_death_given_hosp: SVector::from_iterator(params.p_death_given_hosp),
            inf_death_delay: params.inf_death_delay,
            mitigations: MitigationParamsTyped {
                vaccine,
                antivirals,
                community,
            },
        })
    }
}

impl<const N: usize> From<ParametersTyped<N>> for Parameters {
    fn from(params: ParametersTyped<N>) -> Self {
        let v = params.mitigations.vaccine;
        let a = params.mitigations.antivirals;
        let c = params.mitigations.community;
        Parameters {
            n: N,
            days: params.days,
            population: params.population,
            population_fractions: params.population_fractions.iter().copied().collect(),
            population_fraction_labels: params.population_fraction_labels.iter().cloned().collect(),
            contact_matrix: params.contact_matrix.iter().copied().collect(),
            initial_infections: params.initial_infections,
            fraction_initial_immune: params.fraction_initial_immune,
            r0: params.r0,
            latent_period: params.latent_period,
            infectious_period: params.infectious_period,
            p_symp_given_inf: params.p_symp_given_inf.iter().copied().collect(),
            p_hosp_given_symp: params.p_hosp_given_symp.iter().copied().collect(),
            inf_hosp_delay: params.inf_hosp_delay,
            p_death_given_hosp: params.p_death_given_hosp.iter().copied().collect(),
            inf_death_delay: params.inf_death_delay,

            vaccine_enabled: v.enabled,
            vaccine_doses: v.doses,
            vaccine_start: v.start,
            vaccine_dose2_delay: v.dose2_delay,
            vaccine_p_get_2_doses: v.p_get_2_doses,
            vaccine_administration_rate: v.administration_rate,
            vaccine_doses_available: v.doses_available,
            vaccine_ramp_up: v.ramp_up,
            vaccine_ve_s: v.ve_s,
            vaccine_ve_i: v.ve_i,
            vaccine_ve_p: v.ve_p,
            vaccine_ve_2s: v.ve_2s,
            vaccine_ve_2i: v.ve_2i,
            vaccine_ve_2p: v.ve_2p,

            antivirals_enabled: a.enabled,
            antivirals_fraction_outpatient: a.fraction_outpatient,
            antivirals_fraction_inpatient: a.fraction_inpatient,
            antivirals_ave_i: a.ave_i,
            antivirals_ave_p_hosp: a.ave_p_hosp,
            antivirals_ave_p_death: a.ave_p_death,

            community_enabled: c.enabled,
            community_start: c.start,
            community_duration: c.duration,
            community_effectiveness: c.effectiveness.data.as_slice().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_toml_parses() {
        let params = Parameters::default();
        assert_eq!(params.n, 2);
        assert_eq!(params.population, 330_000_000.0);
        params.validate().expect("default params must validate");
    }

    #[test]
    fn test_has_mitigations() {
        let mut params = Parameters {
            vaccine_enabled: false,
            ..Default::default()
        };
        assert!(!params.has_mitigations());

        params.vaccine_enabled = true;
        assert!(params.has_mitigations());

        params.vaccine_enabled = false;
        params.antivirals_enabled = true;
        assert!(params.has_mitigations());

        params.antivirals_enabled = false;
        params.community_enabled = true;
        assert!(params.has_mitigations());
    }

    #[test]
    fn test_without_mitigations() {
        let params = Parameters {
            vaccine_enabled: true,
            antivirals_enabled: true,
            community_enabled: true,
            ..Default::default()
        };

        let params_no_mitigations = params.without_mitigations();
        assert!(!params_no_mitigations.has_mitigations());
    }

    #[test]
    fn test_typed_roundtrip() {
        // Parameters -> ParametersTyped<2> -> Parameters preserves values.
        let params = Parameters::default();
        let typed: ParametersTyped<2> = params.clone().try_into().unwrap();
        let roundtrip: Parameters = typed.into();
        assert_eq!(params.n, roundtrip.n);
        assert_eq!(params.population_fractions, roundtrip.population_fractions);
        assert_eq!(params.contact_matrix, roundtrip.contact_matrix);
        assert_eq!(
            params.community_effectiveness,
            roundtrip.community_effectiveness
        );
    }

    #[test]
    fn test_contact_matrix_order() {
        // TOML stores the contact matrix as a column-major flat Vec (nalgebra's
        // native storage order). After `SMatrix::from_iterator` the typed
        // matrix must equal the reference default: matrix![18, 3; 9, 12].
        use nalgebra::matrix;
        let typed: ParametersTyped<2> = Parameters::default().try_into().unwrap();
        assert_eq!(typed.contact_matrix, matrix![18.0, 3.0; 9.0, 12.0]);
    }

    #[test]
    fn test_wrong_n_rejected() {
        let params = Parameters::default();
        let r: Result<ParametersTyped<3>, _> = params.try_into();
        assert!(r.is_err());
    }
}
