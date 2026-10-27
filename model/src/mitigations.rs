use nalgebra::SMatrix;

// Solver-internal mitigation carriers. The public `Parameters` struct stores
// mitigation fields flat (e.g. `vaccine_ve_s`); conversion to/from these
// typed carriers happens in `parameters.rs`.

#[derive(Debug, Clone, Copy)]
pub(crate) struct VaccineParams {
    pub enabled: bool,
    pub doses: usize,
    pub start: f64,
    pub dose2_delay: f64,
    pub p_get_2_doses: f64,
    pub administration_rate: f64,
    pub doses_available: f64,
    pub ramp_up: f64,
    pub ve_s: f64,
    pub ve_i: f64,
    pub ve_p: f64,
    pub ve_2s: f64,
    pub ve_2i: f64,
    pub ve_2p: f64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AntiviralsParams {
    pub enabled: bool,
    pub fraction_outpatient: f64,
    pub fraction_inpatient: f64,
    pub ave_i: f64,
    pub ave_p_hosp: f64,
    pub ave_p_death: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct CommunityMitigationParamsTyped<const N: usize> {
    pub enabled: bool,
    pub start: f64,
    pub duration: f64,
    pub effectiveness: SMatrix<f64, N, N>,
}

#[derive(Debug, Clone)]
pub(crate) struct MitigationParamsTyped<const N: usize> {
    pub vaccine: VaccineParams,
    pub antivirals: AntiviralsParams,
    pub community: CommunityMitigationParamsTyped<N>,
}

impl<const N: usize> Default for MitigationParamsTyped<N> {
    fn default() -> Self {
        MitigationParamsTyped {
            vaccine: VaccineParams {
                enabled: false,
                doses: 1,
                dose2_delay: 30.0,
                start: 50.0,
                p_get_2_doses: 0.9,
                administration_rate: 1_500_000.0,
                doses_available: 40_000_000.0,
                ramp_up: 14.0,
                ve_s: 0.40,
                ve_i: 0.0,
                ve_p: 0.5,
                ve_2s: 0.60,
                ve_2i: 0.0,
                ve_2p: 0.75,
            },
            antivirals: AntiviralsParams {
                enabled: false,
                ave_i: 0.30,
                ave_p_hosp: 0.20,
                ave_p_death: 0.1,
                fraction_outpatient: 0.40 * 0.5 * 0.5,
                fraction_inpatient: 1.0,
            },
            community: CommunityMitigationParamsTyped {
                enabled: false,
                start: 60.0,
                duration: 20.0,
                effectiveness: SMatrix::from_element(0.25),
            },
        }
    }
}
