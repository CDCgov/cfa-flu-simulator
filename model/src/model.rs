use crate::model_unified::{DynodeModel, ModelOutput};
use crate::parameters::ParametersTyped;
use nalgebra::{Const, Matrix, MatrixView, SMatrix, SVector, Storage, StorageMut};
use ode_solvers::{Dopri5, System};
use paste::paste;

// `AVE` is the domain acronym used throughout the model docs.
#[allow(clippy::upper_case_acronyms)]
pub struct AVE<const N: usize> {
    pub pop_eff_i_given_symp: SVector<f64, N>,
    pub pop_eff_p_hosp_given_symp: SVector<f64, N>,
    pub pop_eff_p_death_given_hosp: SVector<f64, N>,
}

impl<const N: usize> AVE<N> {
    fn new(params: &ParametersTyped<N>) -> Self {
        let av_params = &params.mitigations.antivirals;
        let zeros = SVector::<f64, N>::from_element(0.0);

        let prob_take_ave_given_symp = av_params.fraction_seek_care
            * av_params.fraction_diagnosed_prescribed_outpatient
            * av_params.fraction_adhere;

        let pop_eff_i_given_symp = if av_params.enabled {
            SVector::<f64, N>::from_element(prob_take_ave_given_symp * av_params.ave_i)
        } else {
            zeros
        };

        let pop_eff_p_hosp_given_symp = if av_params.enabled {
            SVector::<f64, N>::from_element(prob_take_ave_given_symp * av_params.ave_p_hosp)
        } else {
            zeros
        };

        let pop_eff_p_death_given_hosp = if av_params.enabled {
            SVector::<f64, N>::from_element(
                av_params.fraction_diagnosed_prescribed_inpatient * av_params.ave_p_death,
            )
        } else {
            zeros
        };

        Self {
            pop_eff_i_given_symp,
            pop_eff_p_hosp_given_symp,
            pop_eff_p_death_given_hosp,
        }
    }
}

pub struct SEIRModel<const N: usize> {
    pub(crate) parameters: ParametersTyped<N>,
    contact_matrix_normalization: f64,
    ave: AVE<N>,
}

macro_rules! make_state {
    ($( $x:ident),*) => {
        type State<const N: usize> = SVector<f64, { ${count($x)} * N }>;

        trait StateWrapper<const N: usize, S: Storage<f64, Const<{ ${count($x)} * N }>> + 'static>
        where Self: 'static,
        {
            paste! {
            $(
            #[allow(dead_code)]
            fn [<get_  $x>](&self) -> MatrixView<'_, f64, Const<N>, Const<1>, S::RStride, S::CStride>;
            )*
            }
        }

        impl<const N: usize, S: Storage<f64, Const<{ ${count($x)} * N }>> + 'static> StateWrapper<N, S>
            for Matrix<f64, Const<{ ${count($x)} * N }>, Const<1>, S>
        {
            paste! {
            $(
                fn [<get_  $x>](&self) -> MatrixView<'_, f64, Const<N>, Const<1>, S::RStride, S::CStride> {
                    self.fixed_view::<N, 1>(${index()} * N, 0)
                }
            )*
        }

        }

        trait StateWrapperMut<const N: usize>
        where Self: 'static,
        {
            paste! {
            $(
            fn [<set_  $x>]<S: Storage<f64, Const<N>>>(
                &mut self,
                value: &Matrix<f64, Const<N>, Const<1>, S>,
            );
            )*
            }
        }

        impl<const N: usize, S: StorageMut<f64, Const<{ ${count($x)} * N }>> + 'static> StateWrapperMut<N>
        for Matrix<f64, Const<{ ${count($x)} * N }>, Const<1>, S>
        {
            paste! {
            $(
                fn [<set_  $x>]<S2: Storage<f64, Const<N>>>(
                    &mut self,
                    value: &Matrix<f64, Const<N>, Const<1>, S2>,
                ) {
                    self.fixed_view_mut::<N, 1>(${index()} * N, 0).set_column(0, value);
                }
            )*
        }
        }
    }
}

make_state!(
    s, e, i, r, sv, ev, iv, rv, s2v, e2v, i2v, r2v, y_cum, pre_h, h_cum, pre_d, d_cum
);

impl<const N: usize> SEIRModel<N> {
    pub(crate) fn new(parameters: ParametersTyped<N>) -> Self {
        let contact_matrix = parameters.contact_matrix;
        let (eigenvalue, _) = get_dominant_eigendata(&contact_matrix);
        let ave = AVE::new(&parameters);
        SEIRModel {
            parameters,
            contact_matrix_normalization: eigenvalue,
            ave,
        }
    }
}

/// Probability of at least 1 success among N trials each with probability p
pub fn p_detect1(n: f64, p: f64) -> f64 {
    1.0 - (1.0 - p).powf(n)
}

fn distribute_initials<const N: usize>(
    n: SVector<f64, N>,
    seed: SVector<f64, N>,
    r0: SVector<f64, N>,
    e_fraction: f64,
) -> (
    SVector<f64, N>,
    SVector<f64, N>,
    SVector<f64, N>,
    SVector<f64, N>,
) {
    let mut s0_out = SVector::<f64, N>::zeros();
    let mut e0_out = SVector::<f64, N>::zeros();
    let mut i0_out = SVector::<f64, N>::zeros();
    let mut r0_out = SVector::<f64, N>::zeros();

    for j in 0..N {
        let (ss, ee, ii, rr) = _distribute_initials1(n[j], seed[j], r0[j], e_fraction);
        s0_out[j] = ss;
        e0_out[j] = ee;
        i0_out[j] = ii;
        r0_out[j] = rr;
    }

    (s0_out, e0_out, i0_out, r0_out)
}

/// Distribute initial compartment states, accounting for:
/// - split between E and I
/// - initial immunity R
/// - what to do if there aren't enough people to allocate, given `n`, `seed`, and `r0`
fn _distribute_initials1(n: f64, seed: f64, r0: f64, e_fraction: f64) -> (f64, f64, f64, f64) {
    let (s0, seed_allocated, r0_out) = if r0 + seed <= n {
        (n - seed - r0, seed, r0)
    } else if r0 <= n {
        (0.0, n - r0, r0)
    } else {
        panic!("Do not know how to allocate n={n} seed={seed} r0={r0}");
    };
    (
        s0,
        seed_allocated * e_fraction,
        seed_allocated * (1.0 - e_fraction),
        r0_out,
    )
}

/// Contribution of one vaccination group to the effective number of
/// infectious people, discounting transmission averted by the vaccine itself
/// and by outpatient antivirals.
///
/// `ve_i` is effectiveness against onward transmission; `ve_p` is
/// effectiveness against symptoms. Outpatient antivirals only reach
/// symptomatic people, so the antiviral discount applies to the `1 - ve_p`
/// share of the group that still develops symptoms. Unvaccinated people are
/// the `ve_i = ve_p = 0` case of the same formula.
fn effective_infectious<const N: usize, S>(
    infectious: &Matrix<f64, Const<N>, Const<1>, S>,
    ve_i: f64,
    ve_p: f64,
    fraction_symptomatic: &SVector<f64, N>,
    antiviral_reduction_given_symp: &SVector<f64, N>,
) -> SVector<f64, N>
where
    S: Storage<f64, Const<N>, Const<1>>,
{
    let ones = SVector::<f64, N>::from_element(1.0);
    (infectious * (1.0 - ve_i)).component_mul(
        &(ones - (1.0 - ve_p) * fraction_symptomatic.component_mul(antiviral_reduction_given_symp)),
    )
}

fn vaccine_rates_by_dose(
    t: f64,
    max_rate: f64,
    t_start: f64,
    dose2_delay: f64,
    p_get_2_doses: f64,
    doses_available: f64,
) -> (f64, f64) {
    assert!(max_rate >= 0.0);
    assert!(dose2_delay >= 0.0);
    assert!((0.0..=1.0).contains(&p_get_2_doses));
    assert!(doses_available >= 0.0);

    let duration = doses_available / max_rate;

    let t_start1 = t_start;
    let t_end1 = t_start1 + duration;
    let t_start2 = t_start1 + dose2_delay;
    let t_end2 = t_start2 + duration;

    let rate1 = max_rate / (1.0 + p_get_2_doses);
    let rate2 = max_rate - rate1;

    (
        if t_start1 <= t && t < t_end1 {
            rate1
        } else {
            0.0
        },
        if t_start2 <= t && t < t_end2 {
            rate2
        } else {
            0.0
        },
    )
}

impl<const N: usize> DynodeModel for SEIRModel<N>
where
    [(); 17 * N]: Sized,
{
    fn integrate(&self, days: usize) -> ModelOutput {
        let population_fractions = self.parameters.population_fractions;
        let populations = self.parameters.population * population_fractions;

        let mut initial_state: State<N> = SVector::zeros();
        // distribute initial infections between E and I states
        let e_fraction = self.parameters.latent_period
            / (self.parameters.latent_period + self.parameters.infectious_period);
        let (initial_s, initial_e, initial_i, initial_r) = distribute_initials(
            populations,
            self.parameters.initial_infections * population_fractions,
            self.parameters.fraction_initial_immune * populations,
            e_fraction,
        );
        initial_state.set_s(&initial_s);
        initial_state.set_e(&initial_e);
        initial_state.set_i(&initial_i);
        initial_state.set_r(&initial_r);

        // initial I's contribute toward hosps and deaths
        let ones = SVector::<f64, N>::from_element(1.0);
        let initial_pre_h = initial_i
            .component_mul(&self.parameters.fraction_hospitalized)
            .component_mul(&(ones - self.ave.pop_eff_p_hosp_given_symp));
        let initial_pre_d = initial_i
            .component_mul(&self.parameters.fraction_dead)
            .component_mul(&(ones - self.ave.pop_eff_p_hosp_given_symp))
            .component_mul(&(ones - self.ave.pop_eff_p_death_given_hosp));
        initial_state.set_pre_h(&initial_pre_h);
        initial_state.set_pre_d(&initial_pre_d);

        let mut stepper = Dopri5::new(self, 0.0, days as f64, 1.0, initial_state, 1e-6, 1e-6);
        let _res = stepper.integrate();

        let mut output = ModelOutput::new();

        let mut first_loop = true;
        let mut prev_i_plus_r = SVector::zeros();
        let mut prev_iv_plus_rv = SVector::zeros();
        let mut prev_iv2_plus_rv2 = SVector::zeros();
        let mut prev_h_cum = SVector::zeros();
        let mut prev_d_cum = SVector::zeros();

        for (time, state) in stepper.x_out().iter().zip(stepper.y_out().iter()) {
            let i_plus_r = state.get_i() + state.get_r();
            let iv_plus_rv = state.get_iv() + state.get_rv();
            let iv2_plus_rv2 = state.get_i2v() + state.get_r2v();
            if first_loop {
                prev_i_plus_r = i_plus_r;
                prev_iv_plus_rv = iv_plus_rv;
                prev_iv2_plus_rv2 = iv2_plus_rv2;
                prev_h_cum = state.get_h_cum().into();
                prev_d_cum = state.get_d_cum().into();
                first_loop = false;
            } else {
                let new_infections_unvac = i_plus_r - prev_i_plus_r;
                let new_infections_vac = iv_plus_rv - prev_iv_plus_rv;
                let new_infections_vac2 = iv2_plus_rv2 - prev_iv2_plus_rv2;
                let new_infections =
                    new_infections_unvac + new_infections_vac + new_infections_vac2;
                let new_symptomatic = (new_infections_unvac
                    + (1.0 - self.parameters.mitigations.vaccine.ve_p) * new_infections_vac
                    + (1.0 - self.parameters.mitigations.vaccine.ve_2p) * new_infections_vac2)
                    .component_mul(&self.parameters.fraction_symptomatic);
                let new_hospitalizations = state.get_h_cum() - prev_h_cum;
                let new_deaths = state.get_d_cum() - prev_d_cum;
                output.add_infection_incidence(*time, new_infections.data.as_slice().into());
                output.add_symptomatic_incidence(*time, new_symptomatic.data.as_slice().into());
                output.add_hospital_incidence(*time, new_hospitalizations.data.as_slice().into());
                output.add_death_incidence(*time, new_deaths.data.as_slice().into());
                output.add_p_detect(
                    *time,
                    p_detect1(
                        state.get_y_cum().sum() * self.parameters.p_test_sympto,
                        self.parameters.test_sensitivity * self.parameters.p_test_forward,
                    ),
                );
                prev_i_plus_r = i_plus_r;
                prev_iv_plus_rv = iv_plus_rv;
                prev_iv2_plus_rv2 = iv2_plus_rv2;
                prev_h_cum = state.get_h_cum().into();
                prev_d_cum = state.get_d_cum().into();
            }
        }
        output
    }
}

impl<const N: usize> System<f64, State<N>> for &SEIRModel<N> {
    fn system(&self, x: f64, y: &State<N>, dy: &mut State<N>) {
        let s = y.get_s();
        let e = y.get_e();
        let i = y.get_i();
        let r = y.get_r();
        let sv = y.get_sv();
        let ev = y.get_ev();
        let iv = y.get_iv();
        let rv = y.get_rv();
        let s2v = y.get_s2v();
        let e2v = y.get_e2v();
        let i2v = y.get_i2v();
        let pre_h = y.get_pre_h();
        let pre_d = y.get_pre_d();

        let params = &self.parameters;
        let community_params = &params.mitigations.community;
        let vax_params = &params.mitigations.vaccine;

        let contact_matrix = if community_params.enabled
            && x >= community_params.start
            && x < (community_params.start + community_params.duration)
        {
            self.parameters.contact_matrix.component_mul(
                &(SMatrix::<f64, N, N>::from_element(1.0) - community_params.effectiveness),
            ) / self.contact_matrix_normalization
        } else {
            self.parameters.contact_matrix / self.contact_matrix_normalization
        };

        let eff_infectious_period = self.parameters.infectious_period
            * (if params.mitigations.ttiq.enabled {
                (1.0 - params.mitigations.ttiq.p_id_infectious
                    * params.mitigations.ttiq.p_infectious_isolates
                    * params.mitigations.ttiq.isolation_reduction)
                    * (1.0
                        - params.mitigations.ttiq.p_contact_trace
                            * params.mitigations.ttiq.p_traced_quarantines)
            } else {
                1.0
            });

        let beta = self.parameters.r0 / self.parameters.infectious_period;
        let ones = SVector::<f64, N>::from_element(1.0);
        let fraction_symptomatic = &self.parameters.fraction_symptomatic;
        let antiviral_reduction = &self.ave.pop_eff_i_given_symp;
        let i_effective =
            effective_infectious(&i, 0.0, 0.0, fraction_symptomatic, antiviral_reduction)
                + effective_infectious(
                    &iv,
                    vax_params.ve_i,
                    vax_params.ve_p,
                    fraction_symptomatic,
                    antiviral_reduction,
                )
                + effective_infectious(
                    &i2v,
                    vax_params.ve_2i,
                    vax_params.ve_2p,
                    fraction_symptomatic,
                    antiviral_reduction,
                );

        let infection_rate = (beta / self.parameters.population)
            * (contact_matrix * i_effective).component_div(&self.parameters.population_fractions);

        let ds_to_e = s.component_mul(&infection_rate);
        let de_to_i = e / self.parameters.latent_period;
        let di_to_r = i / eff_infectious_period;

        let dsv_to_ev = sv.component_mul(&((1.0 - vax_params.ve_s) * infection_rate));
        let ds2v_to_e2v = s2v.component_mul(&((1.0 - vax_params.ve_2s) * infection_rate));
        let dev_to_iv = ev / self.parameters.latent_period;
        let de2v_to_i2v = e2v / self.parameters.latent_period;
        let div_to_rv = iv / eff_infectious_period;
        let di2v_to_r2v = i2v / eff_infectious_period;

        let (administration_rate, administration_rate2) =
            if vax_params.enabled && vax_params.doses == 1 {
                vaccine_rates_by_dose(
                    x - vax_params.ramp_up,
                    vax_params.administration_rate,
                    vax_params.start,
                    0.0,
                    0.0,
                    vax_params.doses_available,
                )
            } else if vax_params.enabled && vax_params.doses == 2 {
                vaccine_rates_by_dose(
                    x - vax_params.ramp_up,
                    vax_params.administration_rate,
                    vax_params.start,
                    vax_params.dose2_delay,
                    vax_params.p_get_2_doses,
                    vax_params.doses_available,
                )
            } else {
                (0.0, 0.0)
            };
        let u = (s + e + i + r).map(|x| if x == 0.0 { 1.0 } else { x });
        let v = (sv + ev + iv + rv).map(|x| if x == 0.0 { 1.0 } else { x });
        let ds_to_sv = s
            .component_div(&u)
            .component_mul(&self.parameters.population_fractions)
            * administration_rate;
        let dsv_to_s2v = sv
            .component_div(&v)
            .component_mul(&self.parameters.population_fractions)
            * administration_rate2;

        let dat_risk =
            de_to_i + dev_to_iv * (1.0 - vax_params.ve_p) + de2v_to_i2v * (1.0 - vax_params.ve_2p);
        let dsymp = dat_risk.component_mul(&self.parameters.fraction_symptomatic);

        let dto_pre_h = dat_risk
            .component_mul(&self.parameters.fraction_hospitalized)
            .component_mul(&(ones - self.ave.pop_eff_p_hosp_given_symp));
        let dpre_h_to_h_cum = pre_h / self.parameters.hospitalization_delay;

        let dto_pre_d = dat_risk
            .component_mul(&self.parameters.fraction_dead)
            .component_mul(&(ones - self.ave.pop_eff_p_hosp_given_symp))
            .component_mul(&(ones - self.ave.pop_eff_p_death_given_hosp));

        let dpre_d_to_d_cum = pre_d / self.parameters.death_delay;

        dy.set_s(&-(ds_to_e + ds_to_sv));
        dy.set_e(&(ds_to_e - de_to_i));
        dy.set_i(&(de_to_i - di_to_r));
        dy.set_r(&di_to_r);
        dy.set_sv(&(-dsv_to_ev + ds_to_sv - dsv_to_s2v));
        dy.set_ev(&(dsv_to_ev - dev_to_iv));
        dy.set_iv(&(dev_to_iv - div_to_rv));
        dy.set_rv(&div_to_rv);
        dy.set_s2v(&(-ds2v_to_e2v + dsv_to_s2v));
        dy.set_e2v(&(ds2v_to_e2v - de2v_to_i2v));
        dy.set_i2v(&(-di2v_to_r2v + de2v_to_i2v));
        dy.set_r2v(&di2v_to_r2v);
        dy.set_y_cum(&dsymp);
        dy.set_pre_h(&(dto_pre_h - dpre_h_to_h_cum));
        dy.set_h_cum(&dpre_h_to_h_cum);
        dy.set_pre_d(&(dto_pre_d - dpre_d_to_d_cum));
        dy.set_d_cum(&dpre_d_to_d_cum);
    }
}

fn get_dominant_eigendata<const N: usize, S: Storage<f64, Const<N>, Const<N>>>(
    matrix: &Matrix<f64, Const<N>, Const<N>, S>,
) -> (f64, SVector<f64, N>) {
    let mut x = SVector::<f64, N>::from_element(1.0 / N as f64);
    let mut norm = 1.0;
    loop {
        x = matrix * x;
        let new_norm = x.lp_norm(1);
        x /= new_norm;
        if (new_norm - norm).abs() < f64::EPSILON {
            return (norm, x);
        } else {
            norm = new_norm;
        }
    }
}

#[cfg(test)]
mod test {
    use super::SEIRModel;
    use super::{
        _distribute_initials1, distribute_initials, effective_infectious, get_dominant_eigendata,
        vaccine_rates_by_dose,
    };
    use crate::mitigations::{AntiviralsParams, MitigationParamsTyped, TTIQParams, VaccineParams};
    use crate::model_unified::{DynodeModel, ModelOutput, OutputType};
    use crate::parameters::{Parameters, ParametersTyped};
    use float_eq::assert_float_eq;
    use nalgebra::{DVector, Matrix1, Vector1, Vector2, matrix};

    #[derive(Debug)]
    #[allow(dead_code)]
    struct TestResults<const N: usize> {
        pub attack_rate: f64,
        pub symptomatic_rate: f64,
        pub hospitalization_rate: f64,
        pub death_rate: f64,
    }

    impl<const N: usize> TestResults<N> {
        pub fn new(params: &ParametersTyped<N>, output: &ModelOutput) -> Self {
            let total_incidence: f64 = output
                .get_output(&OutputType::InfectionIncidence)
                .iter()
                .map(|x| x.grouped_values.iter().sum::<f64>())
                .sum();

            let symptomatic_incidence: f64 = output
                .get_output(&OutputType::SymptomaticIncidence)
                .iter()
                .map(|x| x.grouped_values.iter().sum::<f64>())
                .sum();

            let hospitalization_incidence: f64 = output
                .get_output(&OutputType::HospitalIncidence)
                .iter()
                .map(|x| x.grouped_values.iter().sum::<f64>())
                .sum();

            let death_incidence: f64 = output
                .get_output(&OutputType::DeathIncidence)
                .iter()
                .map(|x| x.grouped_values.iter().sum::<f64>())
                .sum();

            TestResults {
                attack_rate: total_incidence / params.population,
                symptomatic_rate: symptomatic_incidence / params.population,
                hospitalization_rate: hospitalization_incidence / params.population,
                death_rate: death_incidence / params.population,
            }
        }
    }

    fn default_typed2() -> ParametersTyped<2> {
        Parameters::default().try_into().unwrap()
    }

    #[test]
    fn test_distribute_initials() {
        let n = Vector2::new(100.0, 200.0);
        let i0 = Vector2::new(10.0, 20.0);
        let r0 = Vector2::new(5.0, 10.0);

        let (s0, e0_out, i0_out, r0_out) = distribute_initials(n, i0, r0, 0.4);
        assert_float_eq!(s0[0], 85.0, abs <= 1e-5);
        assert_float_eq!(e0_out[0], 4.0, abs <= 1e-5);
        assert_float_eq!(i0_out[0], 6.0, abs <= 1e-5);
        assert_float_eq!(r0_out[0], 5.0, abs <= 1e-5);
    }

    #[test]
    fn test_distribute_initials_precedence() {
        let (s, e, i, r) = _distribute_initials1(100.0, 75.0, 75.0, 0.4);
        assert_float_eq!(s, 0.0, abs <= 1e-5);
        assert_float_eq!(e, 10.0, abs <= 1e-5);
        assert_float_eq!(i, 15.0, abs <= 1e-5);
        assert_float_eq!(r, 75.0, abs <= 1e-5);
    }

    #[test]
    fn test_seir_immune_equivalent() {
        let fii = 0.25;

        let parameters1 = ParametersTyped {
            population: 330_000_000.0,
            population_fractions: Vector1::new(1.0),
            population_fraction_labels: Vector1::new("All".to_string()),
            contact_matrix: Matrix1::new(1.0),
            initial_infections: 1000.0,
            fraction_initial_immune: fii,
            r0: 2.0,
            latent_period: 1.0,
            infectious_period: 3.0,
            mitigations: MitigationParamsTyped::default(),
            fraction_symptomatic: Vector1::new(0.5),
            fraction_hospitalized: Vector1::new(0.0),
            hospitalization_delay: 1.0,
            fraction_dead: Vector1::new(0.0),
            death_delay: 1.0,
            p_test_sympto: 0.0,
            test_sensitivity: 0.90,
            p_test_forward: 0.90,
        };
        let mut parameters2 = parameters1.clone();
        parameters2.fraction_initial_immune = 0.0;
        parameters2.r0 = parameters1.r0 * (1.0 - fii);
        parameters2.population = parameters1.population * (1.0 - fii);

        let model1 = SEIRModel::new(parameters1);
        let model2 = SEIRModel::new(parameters2);

        let results1 = TestResults::new(&model1.parameters, &model1.integrate(300));
        let results2 = TestResults::new(&model2.parameters, &model2.integrate(300));

        assert_float_eq!(
            results1.attack_rate,
            results2.attack_rate * (1.0 - fii),
            abs <= 1e-10
        );
    }

    #[test]
    fn test_seir_unmitigated() {
        let model = SEIRModel::new(ParametersTyped {
            population: 330_000_000.0,
            population_fractions: Vector1::new(1.0),
            population_fraction_labels: Vector1::new("All".to_string()),
            contact_matrix: Matrix1::new(1.0),
            initial_infections: 1000.0,
            fraction_initial_immune: 0.0,
            r0: 2.0,
            latent_period: 1.0,
            infectious_period: 3.0,
            mitigations: MitigationParamsTyped::default(),
            fraction_symptomatic: Vector1::new(0.5),
            fraction_hospitalized: Vector1::new(0.0),
            hospitalization_delay: 1.0,
            fraction_dead: Vector1::new(0.0),
            death_delay: 1.0,
            p_test_sympto: 0.0,
            test_sensitivity: 0.90,
            p_test_forward: 0.90,
        });
        let results = TestResults::new(&model.parameters, &model.integrate(300));
        assert_float_eq!(results.attack_rate, 0.796814, abs <= 1e-5);
    }

    #[test]
    fn test_seir_vaccine() {
        let vaccine_params = VaccineParams {
            enabled: true,
            editable: true,
            doses: 1,
            start: 0.0,
            administration_rate: 1_000_000.0,
            doses_available: 20_000_000.0,
            ramp_up: 0.0,
            ve_s: 0.5,
            ve_i: 0.5,
            ve_p: 0.5,
            ve_2s: 0.7,
            ve_2i: 0.7,
            ve_2p: 0.7,
            dose2_delay: 0.0,
            p_get_2_doses: 0.0,
        };

        let ttiq_params = TTIQParams {
            enabled: false,
            editable: true,
            p_id_infectious: 0.15,
            p_infectious_isolates: 0.75,
            isolation_reduction: 0.50,
            p_contact_trace: 0.25,
            p_traced_quarantines: 0.75,
        };

        let model = SEIRModel::new(ParametersTyped {
            population: 330_000_000.0,
            population_fractions: Vector1::new(1.0),
            population_fraction_labels: Vector1::new("All".to_string()),
            contact_matrix: Matrix1::new(1.0),
            initial_infections: 1000.0,
            fraction_initial_immune: 0.0,
            r0: 2.0,
            latent_period: 1.0,
            infectious_period: 3.0,
            mitigations: MitigationParamsTyped {
                vaccine: vaccine_params,
                antivirals: MitigationParamsTyped::<1>::default().antivirals,
                community: MitigationParamsTyped::<1>::default().community,
                ttiq: ttiq_params,
            },
            fraction_symptomatic: Vector1::new(0.5),
            fraction_hospitalized: Vector1::new(0.0),
            hospitalization_delay: 1.0,
            fraction_dead: Vector1::new(0.0),
            death_delay: 1.0,
            p_test_sympto: 0.0,
            test_sensitivity: 0.90,
            p_test_forward: 0.90,
        });
        let results = TestResults::new(&model.parameters, &model.integrate(300));
        let expected = 0.7583813;
        assert_float_eq!(results.attack_rate, expected, abs <= 1e-5);
    }

    #[test]
    fn test_seir_perfect_isolation() {
        let mut parameters = default_typed2();
        parameters.mitigations.ttiq = TTIQParams {
            enabled: true,
            editable: true,
            p_id_infectious: 1.0,
            p_infectious_isolates: 1.0,
            isolation_reduction: 1.0,
            p_contact_trace: 0.0,
            p_traced_quarantines: 0.0,
        };

        let model = SEIRModel::new(parameters);
        let results = TestResults::new(&model.parameters, &model.integrate(300));
        assert_float_eq!(results.attack_rate, 0.0, abs <= 1e-10);
    }

    #[test]
    fn test_seir_perfect_quarantine() {
        let mut parameters = default_typed2();
        parameters.mitigations.ttiq = TTIQParams {
            enabled: true,
            editable: true,
            p_id_infectious: 0.0,
            p_infectious_isolates: 0.0,
            isolation_reduction: 0.0,
            p_contact_trace: 1.0,
            p_traced_quarantines: 1.0,
        };

        let model = SEIRModel::new(parameters);
        let results = TestResults::new(&model.parameters, &model.integrate(300));
        assert_float_eq!(results.attack_rate, 0.0, abs <= 1e-10);
    }

    #[test]
    fn final_size_relation_with_groups() {
        let mut params = default_typed2();
        params.mitigations.vaccine.enabled = false;
        params.population = 1.0;
        params.initial_infections = 1e-8;
        params.r0 = 2.0;
        params.latent_period = 1.0;
        params.infectious_period = 3.0;

        let model = SEIRModel::new(params);
        let output = model.integrate(300);

        let total_incidence: f64 = output
            .get_output(&OutputType::InfectionIncidence)
            .iter()
            .map(|x| x.grouped_values.iter().sum::<f64>())
            .sum();
        let attack_rate = total_incidence / model.parameters.population;

        let incidence_by_group = output
            .get_output(&OutputType::InfectionIncidence)
            .iter()
            .map(|x| DVector::from_vec(x.grouped_values.clone()))
            .reduce(|acc, elem| acc + elem)
            .unwrap();
        let attack_rate_by_group = incidence_by_group.component_div(&DVector::from_iterator(
            model.parameters.population_fractions.len(),
            model.parameters.population_fractions.iter().copied(),
        )) / model.parameters.population;

        let hospitalizations_by_group = output
            .get_output(&OutputType::HospitalIncidence)
            .iter()
            .map(|x| DVector::from_vec(x.grouped_values.clone()))
            .reduce(|acc, elem| acc + elem)
            .unwrap();
        let ihr = hospitalizations_by_group.component_div(&incidence_by_group);

        let deaths_by_group = output
            .get_output(&OutputType::DeathIncidence)
            .iter()
            .map(|x| DVector::from_vec(x.grouped_values.clone()))
            .reduce(|acc, elem| acc + elem)
            .unwrap();
        let ifr = deaths_by_group.component_div(&incidence_by_group);

        assert!((0.6755054 - attack_rate).abs() < 1e-5);

        assert!((0.8658730 - attack_rate_by_group[0]).abs() < 1e-5);
        assert!((0.6120495 - attack_rate_by_group[1]).abs() < 1e-5);

        assert!((model.parameters.fraction_hospitalized[0] - ihr[0]).abs() < 1e-5);
        assert!((model.parameters.fraction_hospitalized[1] - ihr[1]).abs() < 1e-5);

        assert!(
            (model.parameters.fraction_dead[0] - ifr[0]).abs() < 1e-5,
            "fraction_dead={:?} ifr={:?}",
            model.parameters.fraction_dead,
            ifr
        );
        assert!((model.parameters.fraction_dead[1] - ifr[1]).abs() < 1e-5);
    }

    #[test]
    fn test_antiviral() {
        let mut params = ParametersTyped {
            population: 330_000_000.0,
            population_fractions: Vector1::new(1.0),
            population_fraction_labels: Vector1::new("All".to_string()),
            contact_matrix: Matrix1::new(1.0),
            initial_infections: 1_000.0,
            fraction_initial_immune: 0.0,
            r0: 2.0,
            latent_period: 1.0,
            infectious_period: 3.0,
            mitigations: MitigationParamsTyped::default(),
            fraction_symptomatic: Vector1::new(0.5),
            fraction_hospitalized: Vector1::new(0.1),
            hospitalization_delay: 1.0,
            fraction_dead: Vector1::new(0.01),
            death_delay: 1.0,
            p_test_sympto: 0.0,
            test_sensitivity: 0.90,
            p_test_forward: 0.90,
        };
        params.mitigations.antivirals = AntiviralsParams {
            enabled: true,
            editable: true,
            ave_i: 0.5,
            ave_p_hosp: 0.5,
            ave_p_death: 0.0,
            fraction_adhere: 0.5,
            fraction_diagnosed_prescribed_inpatient: 0.5,
            fraction_diagnosed_prescribed_outpatient: 0.5,
            fraction_seek_care: 0.5,
        };

        let model = SEIRModel::new(params);
        let results = TestResults::new(&model.parameters, &model.integrate(300));
        assert_float_eq!(results.attack_rate, 0.77889514, abs <= 1e-5);
    }

    /// One vaccination group against a fixed backdrop: half of it would be
    /// symptomatic, and an outpatient antiviral programme averts 40% of
    /// transmission from those it reaches.
    fn test_effective_infectious(infectious: f64, ve_i: f64, ve_p: f64) -> f64 {
        effective_infectious(
            &Vector1::new(infectious),
            ve_i,
            ve_p,
            &Vector1::new(0.5),
            &Vector1::new(0.4),
        )[0]
    }

    #[test]
    fn test_unvaccinated_gets_full_antiviral_discount() {
        // Every symptomatic person is reachable: 100 * (1 - 0.5 * 0.4) = 80.
        assert_eq!(test_effective_infectious(100.0, 0.0, 0.0), 80.0);
    }

    #[test]
    fn test_full_symptom_protection_forgoes_antivirals() {
        // ve_p = 1: nobody here develops symptoms, so no outpatient antiviral
        // reaches them and the discount vanishes entirely.
        assert_eq!(test_effective_infectious(100.0, 0.0, 1.0), 100.0);
    }

    #[test]
    fn test_effective_infectious_increases_with_ve_p() {
        // Symptom protection trades away antiviral coverage, so effective
        // infectiousness rises with ve_p rather than falling.
        let effs: Vec<f64> = [0.0, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .map(|ve_p| test_effective_infectious(100.0, 0.0, *ve_p))
            .collect();

        for pair in effs.windows(2) {
            assert!(
                pair[1] > pair[0],
                "effective infectiousness must increase with ve_p \
                 (antivirals cannot reach asymptomatic people), got {effs:?}"
            );
        }
    }

    #[test]
    fn test_transmission_protection_scales_linearly() {
        // ve_i applies to the whole group, independently of the antiviral
        // discount: 100 * 0.25 * (1 - 0.5 * 0.4) = 20.
        assert_eq!(test_effective_infectious(100.0, 0.75, 0.0), 20.0);
    }

    #[test]
    fn test_symptom_protection_forgoes_outpatient_antivirals() {
        // End-to-end guard that the per-group formula is wired into the solver:
        // at the shared TOML defaults, varying only ve_p, the attack rate must
        // rise with ve_p once antivirals are on.
        let attack_rate_at_ve_p = |ve_p: f64| {
            let p = Parameters {
                vaccine_enabled: true,
                antivirals_enabled: true,
                vaccine_ve_p: ve_p,
                ..Default::default()
            };

            let params: ParametersTyped<2> = p.try_into().unwrap();
            let model = SEIRModel::new(params);
            TestResults::new(&model.parameters, &model.integrate(300)).attack_rate
        };

        let rates: Vec<f64> = [0.0, 0.5, 1.0]
            .iter()
            .map(|v| attack_rate_at_ve_p(*v))
            .collect();
        for pair in rates.windows(2) {
            assert!(
                pair[1] > pair[0],
                "attack rate must increase with ve_p when antivirals are on, got {rates:?}"
            );
        }
    }

    #[test]
    fn test_2dose_vaccine() {
        let mut params = ParametersTyped {
            population: 330_000_000.0,
            population_fractions: Vector1::new(1.0),
            population_fraction_labels: Vector1::new("All".to_string()),
            contact_matrix: Matrix1::new(1.0),
            initial_infections: 1_000.0,
            fraction_initial_immune: 0.0,
            r0: 2.0,
            latent_period: 1.0,
            infectious_period: 3.0,
            mitigations: MitigationParamsTyped::default(),
            fraction_symptomatic: Vector1::new(0.5),
            fraction_hospitalized: Vector1::new(0.1),
            hospitalization_delay: 1.0,
            fraction_dead: Vector1::new(0.01),
            death_delay: 1.0,
            p_test_sympto: 0.0,
            test_sensitivity: 0.90,
            p_test_forward: 0.90,
        };
        params.mitigations.vaccine = VaccineParams {
            enabled: true,
            editable: true,
            doses: 2,
            start: 0.0,
            dose2_delay: 0.0,
            p_get_2_doses: 1.0,
            administration_rate: 1_000_000.0,
            doses_available: 20_000_000.0,
            ramp_up: 0.0,
            ve_s: 0.50,
            ve_i: 0.50,
            ve_p: 0.50,
            ve_2s: 0.75,
            ve_2i: 0.75,
            ve_2p: 0.75,
        };

        let model = SEIRModel::new(params);
        let results = TestResults::new(&model.parameters, &model.integrate(300));
        assert_float_eq!(results.attack_rate, 0.7672022, abs <= 1e-5);
    }

    #[test]
    fn test_2dose_vaccine_ignore_dose1() {
        let mut params1 = ParametersTyped {
            population: 330_000_000.0,
            population_fractions: Vector1::new(1.0),
            population_fraction_labels: Vector1::new("All".to_string()),
            contact_matrix: Matrix1::new(1.0),
            initial_infections: 1_000.0,
            fraction_initial_immune: 0.0,
            r0: 2.0,
            latent_period: 1.0,
            infectious_period: 3.0,
            mitigations: MitigationParamsTyped::default(),
            fraction_symptomatic: Vector1::new(0.5),
            fraction_hospitalized: Vector1::new(0.1),
            hospitalization_delay: 1.0,
            fraction_dead: Vector1::new(0.01),
            death_delay: 1.0,
            p_test_sympto: 0.0,
            test_sensitivity: 0.90,
            p_test_forward: 0.90,
        };
        let vax_params1 = VaccineParams {
            enabled: true,
            editable: true,
            doses: 2,
            start: 0.0,
            dose2_delay: 0.0,
            p_get_2_doses: 0.0,
            administration_rate: 1_000_000.0,
            doses_available: 20_000_000.0,
            ramp_up: 0.0,
            ve_s: 0.50,
            ve_i: 0.50,
            ve_p: 0.50,
            ve_2s: 0.75,
            ve_2i: 0.75,
            ve_2p: 0.75,
        };
        params1.mitigations.vaccine = vax_params1;

        let mut params2 = params1.clone();
        let mut vax_params2 = vax_params1;
        vax_params2.doses = 1;
        vax_params2.p_get_2_doses = 1.0 / 3.0;
        params2.mitigations.vaccine = vax_params2;

        let model1 = SEIRModel::new(params1);
        let model2 = SEIRModel::new(params2);

        let results1 = TestResults::new(&model1.parameters, &model1.integrate(300));
        let results2 = TestResults::new(&model2.parameters, &model2.integrate(300));

        assert_float_eq!(results1.attack_rate, results2.attack_rate, abs <= 1e-10);
    }

    #[test]
    fn test_eigen() {
        let x = matrix![1.0, 3.0; 2.0, 4.0];
        let (eval, evec) = get_dominant_eigendata(&x);
        assert!((eval - 5.3722813).abs() < 1e-6);
        assert!((evec[0] - 0.4069297).abs() < 1e-6);
        assert!((evec[1] - 0.5930703).abs() < 1e-6);
    }

    #[test]
    fn test_vax_rate_by_dose() {
        let (rate1, rate2) = vaccine_rates_by_dose(0.0, 1.0, 1.0, 0.0, 1.0, 10.0);
        assert_float_eq!(rate1, 0.0, abs <= 1e-6);
        assert_float_eq!(rate2, 0.0, abs <= 1e-6);

        let (rate1, rate2) = vaccine_rates_by_dose(0.0, 1.0, 0.0, 1.0, 0.0, 10.0);
        assert_float_eq!(rate1, 1.0, abs <= 1e-6);
        assert_float_eq!(rate2, 0.0, abs <= 1e-6);

        let (rate1, rate2) = vaccine_rates_by_dose(0.0, 1.0, 0.0, 0.0, 1.0, 10.0);
        assert_float_eq!(rate1, 0.5, abs <= 1e-6);
        assert_float_eq!(rate2, 0.5, abs <= 1e-6);

        let (rate1, rate2) = vaccine_rates_by_dose(0.1, 1.0, 0.0, 0.0, 0.9, 10.0);
        assert_float_eq!(rate1, 1.0 / 1.9, abs <= 1e-6);
        assert_float_eq!(rate2, 0.9 * 1.0 / 1.9, abs <= 1e-6);

        let (rate1, rate2) = vaccine_rates_by_dose(7.5, 1.0, 0.0, 5.0, 1.0, 10.0);
        assert_float_eq!(rate1, 0.5, abs <= 1e-6);
        assert_float_eq!(rate2, 0.5, abs <= 1e-6);
    }

    #[test]
    fn test_ramp_up() {
        let mut params1 = default_typed2();
        let mut vax_params1 = params1.mitigations.vaccine;
        vax_params1.enabled = true;
        vax_params1.start = 0.0;
        vax_params1.ramp_up = 14.0;
        params1.mitigations.vaccine = vax_params1;

        let mut params2 = default_typed2();
        let mut vax_params2 = vax_params1;
        vax_params2.enabled = true;
        vax_params2.start = 14.0;
        vax_params2.ramp_up = 0.0;
        params2.mitigations.vaccine = vax_params2;

        let model1 = SEIRModel::new(params1);
        let model2 = SEIRModel::new(params2);

        let sim_duration = 300;
        let results1 = TestResults::new(&model1.parameters, &model1.integrate(sim_duration));
        let results2 = TestResults::new(&model2.parameters, &model2.integrate(sim_duration));

        assert_float_eq!(results1.attack_rate, results2.attack_rate, abs <= 1e-10);
    }
}
