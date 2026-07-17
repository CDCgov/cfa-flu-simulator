use crate::model_unified::{EpidemicModel, ModelOutput};
use crate::parameters::ParametersTyped;
use nalgebra::{Const, Matrix, MatrixView, SMatrix, SVector, Storage, StorageMut};
use ode_solvers::{Dopri5, System};
use paste::paste;

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

    #[cfg(test)]
    pub(crate) fn integrate_living_from_state(
        &self,
        days: usize,
        living: &[f64],
    ) -> Vec<(f64, Vec<f64>)>
    where
        [(); 17 * N]: Sized,
    {
        assert_eq!(living.len(), 12 * N);
        let mut initial_state: State<N> = SVector::zeros();
        initial_state.as_mut_slice()[..12 * N].copy_from_slice(living);
        let mut stepper = Dopri5::new(self, 0.0, days as f64, 1.0, initial_state, 1e-6, 1e-6);
        stepper.integrate().expect("reference integration succeeds");
        stepper
            .x_out()
            .iter()
            .zip(stepper.y_out())
            .map(|(time, state)| (*time, state.as_slice()[..12 * N].to_vec()))
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn living_derivative_at(&self, time: f64, living: &[f64]) -> Vec<f64>
    where
        [(); 17 * N]: Sized,
    {
        assert_eq!(living.len(), 12 * N);
        let mut state: State<N> = SVector::zeros();
        state.as_mut_slice()[..12 * N].copy_from_slice(living);
        let mut derivative: State<N> = SVector::zeros();
        System::system(&self, time, &state, &mut derivative);
        derivative.as_slice()[..12 * N].to_vec()
    }
}

/// Probability of at least 1 success among N trials each with probability p
pub fn p_detect1(n: f64, p: f64) -> f64 {
    1.0 - (1.0 - p).powf(n)
}

fn distribute_initials<const N: usize>(
    n: SVector<f64, N>,
    i0: SVector<f64, N>,
    r0: SVector<f64, N>,
) -> (SVector<f64, N>, SVector<f64, N>, SVector<f64, N>) {
    let mut s0_out = SVector::<f64, N>::zeros();
    let mut i0_out = SVector::<f64, N>::zeros();
    let mut r0_out = SVector::<f64, N>::zeros();

    for j in 0..N {
        let (ss, ii, rr) = _distribute_initials1(n[j], i0[j], r0[j]);
        s0_out[j] = ss;
        i0_out[j] = ii;
        r0_out[j] = rr;
    }

    (s0_out, i0_out, r0_out)
}

fn _distribute_initials1(n: f64, i0: f64, r0: f64) -> (f64, f64, f64) {
    if r0 + i0 <= n {
        (n - i0 - r0, i0, r0)
    } else if r0 <= n {
        (0.0, n - r0, r0)
    } else {
        panic!("Do not know how to allocate n={n} i0={i0} r0={r0}");
    }
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

impl<const N: usize> EpidemicModel for SEIRModel<N>
where
    [(); 17 * N]: Sized,
{
    fn integrate(&self, days: usize) -> ModelOutput {
        let population_fractions = self.parameters.population_fractions;
        let populations = self.parameters.population * population_fractions;

        let mut initial_state: State<N> = SVector::zeros();
        let (initial_s, initial_i, initial_r) = distribute_initials(
            populations,
            self.parameters.initial_infections * population_fractions,
            self.parameters.fraction_initial_immune * populations,
        );
        initial_state.set_s(&initial_s);
        initial_state.set_i(&initial_i);
        initial_state.set_r(&initial_r);

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
        let i_effective = i.component_mul(
            &(ones
                - self
                    .parameters
                    .fraction_symptomatic
                    .component_mul(&self.ave.pop_eff_i_given_symp)),
        ) + (iv * (1.0 - vax_params.ve_i)).component_mul(
            &(ones
                - vax_params.ve_p
                    * self
                        .parameters
                        .fraction_symptomatic
                        .component_mul(&self.ave.pop_eff_i_given_symp)),
        ) + (i2v * (1.0 - vax_params.ve_2i)).component_mul(
            &(ones
                - vax_params.ve_2p
                    * self
                        .parameters
                        .fraction_symptomatic
                        .component_mul(&self.ave.pop_eff_i_given_symp)),
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
        _distribute_initials1, State, distribute_initials, get_dominant_eigendata, p_detect1,
        vaccine_rates_by_dose,
    };
    use crate::mitigations::{AntiviralsParams, MitigationParamsTyped, TTIQParams, VaccineParams};
    use crate::model_unified::{EpidemicModel, ModelOutput, OutputType};
    use crate::parameters::{Parameters, ParametersTyped};
    use float_eq::assert_float_eq;
    use nalgebra::{DVector, Matrix1, SVector, Vector1, Vector2, matrix};
    use ode_solvers::Dopri5;

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

    fn one_group_params() -> ParametersTyped<1> {
        ParametersTyped {
            population: 1_000_000.0,
            population_fractions: Vector1::new(1.0),
            population_fraction_labels: Vector1::new("All".to_string()),
            contact_matrix: Matrix1::new(1.0),
            initial_infections: 0.01,
            fraction_initial_immune: 0.0,
            r0: 2.0,
            latent_period: 1.0,
            infectious_period: 3.0,
            mitigations: MitigationParamsTyped::default(),
            fraction_symptomatic: Vector1::new(0.5),
            fraction_hospitalized: Vector1::new(0.1),
            hospitalization_delay: 7.0,
            fraction_dead: Vector1::new(0.01),
            death_delay: 10.0,
            p_test_sympto: 0.001,
            test_sensitivity: 0.9,
            p_test_forward: 0.9,
        }
    }

    fn total(output: &ModelOutput, output_type: OutputType) -> f64 {
        output
            .get_output(&output_type)
            .iter()
            .flat_map(|row| row.grouped_values.iter())
            .sum()
    }

    fn assert_attack_rate(params: ParametersTyped<1>, days: usize, expected: f64, tolerance: f64) {
        let model = SEIRModel::new(params);
        let actual = TestResults::new(&model.parameters, &model.integrate(days)).attack_rate;
        assert_float_eq!(actual, expected, abs <= tolerance);
    }

    fn assert_vaccine_rates(
        input: (f64, f64, f64, f64, f64, f64),
        expected: (f64, f64),
        tolerance: f64,
    ) {
        let (t, max_rate, start, dose2_delay, second_dose_uptake, supply) = input;
        let actual =
            vaccine_rates_by_dose(t, max_rate, start, dose2_delay, second_dose_uptake, supply);
        assert_float_eq!(actual.0, expected.0, abs <= tolerance);
        assert_float_eq!(actual.1, expected.1, abs <= tolerance);
    }

    fn assert_stronger_mitigation_reduces_burden(
        weak: ParametersTyped<1>,
        strong: ParametersTyped<1>,
    ) {
        let attack_rate = |params| {
            let model = SEIRModel::new(params);
            TestResults::new(&model.parameters, &model.integrate(400)).attack_rate
        };
        assert!(attack_rate(strong) < attack_rate(weak) - 1e-6);
    }

    fn assert_outputs_close(actual: &ModelOutput, expected: &ModelOutput, abs: f64, rel: f64) {
        for output_type in [
            OutputType::InfectionIncidence,
            OutputType::SymptomaticIncidence,
            OutputType::HospitalIncidence,
            OutputType::DeathIncidence,
        ] {
            let actual_rows = actual.get_output(&output_type);
            let expected_rows = expected.get_output(&output_type);
            assert_eq!(actual_rows.len(), expected_rows.len());
            for (a, e) in actual_rows.iter().zip(expected_rows) {
                assert_eq!(a.time, e.time);
                for (av, ev) in a.grouped_values.iter().zip(&e.grouped_values) {
                    let tolerance = abs.max(rel * ev.abs());
                    assert!(
                        (av - ev).abs() <= tolerance,
                        "{output_type:?} at t={}: {av} != {ev} (tol={tolerance})",
                        a.time
                    );
                }
            }
        }
    }

    #[test]
    fn test_distribute_initials() {
        let n = Vector2::new(100.0, 200.0);
        let i0 = Vector2::new(10.0, 20.0);
        let r0 = Vector2::new(5.0, 10.0);

        let (s0, i0_out, r0_out) = distribute_initials(n, i0, r0);
        assert_float_eq!(s0[0], 85.0, abs <= 1e-5);
        assert_float_eq!(i0_out[0], 10.0, abs <= 1e-5);
        assert_float_eq!(r0_out[0], 5.0, abs <= 1e-5);
    }

    #[test]
    fn test_distribute_initials_precedence() {
        let (s, i, r) = _distribute_initials1(100.0, 75.0, 75.0);
        assert_float_eq!(s, 0.0, abs <= 1e-5);
        assert_float_eq!(i, 25.0, abs <= 1e-5);
        assert_float_eq!(r, 75.0, abs <= 1e-5);
    }

    #[test]
    fn test_seir_immune_equivalent() {
        let fii = 0.25;

        let mut parameters1 = one_group_params();
        parameters1.population = 330_000_000.0;
        parameters1.initial_infections = 1000.0;
        parameters1.fraction_initial_immune = fii;
        parameters1.fraction_hospitalized = Vector1::new(0.0);
        parameters1.fraction_dead = Vector1::new(0.0);
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
    fn test_unmitigated_final_size_equation() {
        let params = one_group_params();
        let initial_infected = params.initial_infections / params.population;
        let susceptible0 = 1.0 - initial_infected;
        let r0 = params.r0;
        let model = SEIRModel::new(params);
        let actual = TestResults::new(&model.parameters, &model.integrate(500)).attack_rate;

        // ln(S_inf / S_0) + R0 * (1 - S_inf) = 0. The nontrivial root lies
        // between zero and S_0 for R0 > 1.
        let residual = |s_inf: f64| (s_inf / susceptible0).ln() + r0 * (1.0 - s_inf);
        let (mut low, mut high) = (f64::MIN_POSITIVE, susceptible0 * (1.0 - 1e-12));
        for _ in 0..200 {
            let middle = (low + high) / 2.0;
            if residual(middle) < 0.0 {
                low = middle;
            } else {
                high = middle;
            }
        }
        let susceptible_inf = (low + high) / 2.0;
        let expected = susceptible0 - susceptible_inf;
        assert!(residual(susceptible_inf).abs() <= 1e-12);
        assert_float_eq!(actual, expected, abs <= 1e-5);
    }

    #[test]
    fn test_zero_initial_infections_zero_outputs() {
        let mut params = one_group_params();
        params.initial_infections = 0.0;
        params.mitigations.vaccine.enabled = true;
        params.mitigations.antivirals.enabled = true;
        params.mitigations.community.enabled = true;
        params.mitigations.ttiq.enabled = true;
        let model = SEIRModel::new(params);
        let output = model.integrate(200);
        for output_type in [
            OutputType::InfectionIncidence,
            OutputType::SymptomaticIncidence,
            OutputType::HospitalIncidence,
            OutputType::DeathIncidence,
        ] {
            assert!(total(&output, output_type).abs() <= 1e-12);
        }
    }

    #[test]
    fn test_detection_probability() {
        assert_float_eq!(p_detect1(0.0, 0.75), 0.0, abs <= 1e-12);
        assert_float_eq!(p_detect1(10.0, 0.0), 0.0, abs <= 1e-12);
        assert_float_eq!(p_detect1(1.0, 0.25), 0.25, abs <= 1e-12);
        assert_float_eq!(p_detect1(2.0, 0.5), 0.75, abs <= 1e-12);
        assert_float_eq!(p_detect1(10.0, 1.0), 1.0, abs <= 1e-12);
        assert!(p_detect1(10.0, 0.2) > p_detect1(5.0, 0.2));
        assert!(p_detect1(10.0, 0.3) > p_detect1(10.0, 0.2));
    }

    #[test]
    fn test_population_conservation_and_nonnegative_states() {
        let mut params = default_typed2();
        params.mitigations.vaccine.enabled = true;
        params.mitigations.antivirals.enabled = true;
        params.mitigations.community.enabled = true;
        params.mitigations.ttiq.enabled = true;
        let model = SEIRModel::new(params);

        let populations = model.parameters.population * model.parameters.population_fractions;
        let (s0, i0, r0) = distribute_initials(
            populations,
            model.parameters.initial_infections * model.parameters.population_fractions,
            model.parameters.fraction_initial_immune * populations,
        );
        let mut initial_state: State<2> = SVector::zeros();
        super::StateWrapperMut::set_s(&mut initial_state, &s0);
        super::StateWrapperMut::set_i(&mut initial_state, &i0);
        super::StateWrapperMut::set_r(&mut initial_state, &r0);
        let mut stepper = Dopri5::new(&model, 0.0, 300.0, 1.0, initial_state, 1e-6, 1e-6);
        stepper
            .integrate()
            .expect("representative integration succeeds");

        for (time, state) in stepper.x_out().iter().zip(stepper.y_out()) {
            let living = &state.as_slice()[..24];
            let sum: f64 = living.iter().sum();
            let tolerance = 1e-7 * model.parameters.population;
            assert!(
                (sum - model.parameters.population).abs() <= tolerance,
                "population not conserved at t={time}: {sum}"
            );
            assert!(
                living.iter().all(|value| *value >= -tolerance),
                "negative living compartment at t={time}"
            );
        }
    }

    #[test]
    fn test_seir_unmitigated() {
        let mut params = one_group_params();
        params.population = 330_000_000.0;
        params.initial_infections = 1000.0;
        params.fraction_hospitalized = Vector1::new(0.0);
        params.fraction_dead = Vector1::new(0.0);
        assert_attack_rate(params, 300, 0.796814, 1e-5);
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

        let mut params = one_group_params();
        params.population = 330_000_000.0;
        params.initial_infections = 1000.0;
        params.fraction_hospitalized = Vector1::new(0.0);
        params.fraction_dead = Vector1::new(0.0);
        params.mitigations = MitigationParamsTyped {
            vaccine: vaccine_params,
            antivirals: MitigationParamsTyped::<1>::default().antivirals,
            community: MitigationParamsTyped::<1>::default().community,
            ttiq: ttiq_params,
        };
        assert_attack_rate(params, 300, 0.7583813, 1e-5);
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
        let mut params = one_group_params();
        params.population = 330_000_000.0;
        params.initial_infections = 1_000.0;
        params.hospitalization_delay = 1.0;
        params.death_delay = 1.0;
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

        assert_attack_rate(params, 300, 0.77889514, 1e-5);
    }

    #[test]
    fn test_2dose_vaccine() {
        let mut params = one_group_params();
        params.population = 330_000_000.0;
        params.initial_infections = 1_000.0;
        params.hospitalization_delay = 1.0;
        params.death_delay = 1.0;
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

        assert_attack_rate(params, 300, 0.7672022, 1e-5);
    }

    #[test]
    fn test_2dose_vaccine_ignore_dose1() {
        let mut params1 = one_group_params();
        params1.population = 330_000_000.0;
        params1.initial_infections = 1_000.0;
        params1.hospitalization_delay = 1.0;
        params1.death_delay = 1.0;
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
    fn test_vax_rate_before_campaign() {
        assert_vaccine_rates((0.0, 1.0, 1.0, 0.0, 1.0, 10.0), (0.0, 0.0), 1e-6);
    }

    #[test]
    fn test_vax_rate_one_dose() {
        assert_vaccine_rates((0.0, 1.0, 0.0, 1.0, 0.0, 10.0), (1.0, 0.0), 1e-6);
    }

    #[test]
    fn test_vax_rate_simultaneous_doses() {
        assert_vaccine_rates((0.0, 1.0, 0.0, 0.0, 1.0, 10.0), (0.5, 0.5), 1e-6);
    }

    #[test]
    fn test_vax_rate_partial_second_dose_uptake() {
        assert_vaccine_rates(
            (0.1, 1.0, 0.0, 0.0, 0.9, 10.0),
            (1.0 / 1.9, 0.9 / 1.9),
            1e-6,
        );
    }

    #[test]
    fn test_vax_rate_after_second_dose_delay() {
        assert_vaccine_rates((7.5, 1.0, 0.0, 5.0, 1.0, 10.0), (0.5, 0.5), 1e-6);
    }

    #[test]
    fn test_vax_rate_depletion_gap() {
        assert_vaccine_rates((15.0, 1.0, 0.0, 100.0, 1.0, 10.0), (0.0, 0.0), 1e-12);
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

    #[test]
    fn test_zero_effect_mitigations_equivalent() {
        let mut baseline_params = one_group_params();
        baseline_params.population = 330_000_000.0;
        baseline_params.initial_infections = 3.3;
        let baseline_model = SEIRModel::new(baseline_params.clone());
        let baseline = baseline_model.integrate(300);

        let mut cases = Vec::new();

        let mut vaccine = baseline_params.clone();
        vaccine.mitigations.vaccine.enabled = true;
        vaccine.mitigations.vaccine.start = 0.0;
        vaccine.mitigations.vaccine.ramp_up = 0.0;
        vaccine.mitigations.vaccine.administration_rate = 3_300_000.0;
        vaccine.mitigations.vaccine.doses_available = 165_000_000.0;
        vaccine.mitigations.vaccine.ve_s = 0.0;
        vaccine.mitigations.vaccine.ve_i = 0.0;
        vaccine.mitigations.vaccine.ve_p = 0.0;
        vaccine.mitigations.vaccine.ve_2s = 0.0;
        vaccine.mitigations.vaccine.ve_2i = 0.0;
        vaccine.mitigations.vaccine.ve_2p = 0.0;
        cases.push(vaccine);

        let mut antivirals = baseline_params.clone();
        antivirals.mitigations.antivirals.enabled = true;
        antivirals.mitigations.antivirals.ave_i = 0.0;
        antivirals.mitigations.antivirals.ave_p_hosp = 0.0;
        antivirals.mitigations.antivirals.ave_p_death = 0.0;
        cases.push(antivirals);

        let mut community = baseline_params.clone();
        community.mitigations.community.enabled = true;
        community.mitigations.community.start = 0.0;
        community.mitigations.community.duration = 300.0;
        community.mitigations.community.effectiveness = Matrix1::new(0.0);
        cases.push(community);

        let mut ttiq = baseline_params;
        ttiq.mitigations.ttiq.enabled = true;
        ttiq.mitigations.ttiq.p_id_infectious = 0.0;
        ttiq.mitigations.ttiq.p_contact_trace = 0.0;
        cases.push(ttiq);

        for params in cases {
            let actual = SEIRModel::new(params).integrate(300);
            assert_outputs_close(
                &actual,
                &baseline,
                2e-7 * baseline_model.parameters.population,
                1e-6,
            );
        }
    }

    #[test]
    fn test_r0_increases_infection_burden() {
        let mut low = one_group_params();
        low.r0 = 1.2;
        let mut high = low.clone();
        high.r0 = 2.0;
        let low_model = SEIRModel::new(low);
        let high_model = SEIRModel::new(high);
        let low_rate =
            TestResults::new(&low_model.parameters, &low_model.integrate(500)).attack_rate;
        let high_rate =
            TestResults::new(&high_model.parameters, &high_model.integrate(500)).attack_rate;
        assert!(high_rate > low_rate + 1e-6);
    }

    #[test]
    fn test_stronger_mitigations_reduce_targeted_burden() {
        let mut weak_vaccine = one_group_params();
        weak_vaccine.mitigations.vaccine.enabled = true;
        weak_vaccine.mitigations.vaccine.start = 0.0;
        weak_vaccine.mitigations.vaccine.ramp_up = 0.0;
        weak_vaccine.mitigations.vaccine.administration_rate = 10_000.0;
        weak_vaccine.mitigations.vaccine.doses_available = 500_000.0;
        weak_vaccine.mitigations.vaccine.ve_s = 0.0;
        let mut strong_vaccine = weak_vaccine.clone();
        strong_vaccine.mitigations.vaccine.ve_s = 0.8;
        assert_stronger_mitigation_reduces_burden(weak_vaccine, strong_vaccine);

        let mut weak_antiviral = one_group_params();
        weak_antiviral.mitigations.antivirals.enabled = true;
        weak_antiviral.mitigations.antivirals.fraction_seek_care = 1.0;
        weak_antiviral
            .mitigations
            .antivirals
            .fraction_diagnosed_prescribed_outpatient = 1.0;
        weak_antiviral.mitigations.antivirals.fraction_adhere = 1.0;
        weak_antiviral.mitigations.antivirals.ave_i = 0.0;
        let mut strong_antiviral = weak_antiviral.clone();
        strong_antiviral.mitigations.antivirals.ave_i = 0.8;
        assert_stronger_mitigation_reduces_burden(weak_antiviral, strong_antiviral);

        let mut weak_community = one_group_params();
        weak_community.mitigations.community.enabled = true;
        weak_community.mitigations.community.start = 0.0;
        weak_community.mitigations.community.duration = 400.0;
        weak_community.mitigations.community.effectiveness = Matrix1::new(0.0);
        let mut strong_community = weak_community.clone();
        strong_community.mitigations.community.effectiveness = Matrix1::new(0.5);
        assert_stronger_mitigation_reduces_burden(weak_community, strong_community);

        let mut weak_ttiq = one_group_params();
        weak_ttiq.mitigations.ttiq.enabled = true;
        weak_ttiq.mitigations.ttiq.p_id_infectious = 0.0;
        weak_ttiq.mitigations.ttiq.p_contact_trace = 0.0;
        let mut strong_ttiq = weak_ttiq.clone();
        strong_ttiq.mitigations.ttiq.p_id_infectious = 1.0;
        strong_ttiq.mitigations.ttiq.p_infectious_isolates = 1.0;
        strong_ttiq.mitigations.ttiq.isolation_reduction = 0.5;
        assert_stronger_mitigation_reduces_burden(weak_ttiq, strong_ttiq);
    }

    #[test]
    fn test_ttiq_equivalent_parameterization() {
        let mut with_ttiq = one_group_params();
        with_ttiq.mitigations.ttiq = TTIQParams {
            enabled: true,
            editable: true,
            p_id_infectious: 0.5,
            p_infectious_isolates: 0.8,
            isolation_reduction: 0.5,
            p_contact_trace: 0.25,
            p_traced_quarantines: 0.4,
        };
        let factor = (1.0 - 0.5 * 0.8 * 0.5) * (1.0 - 0.25 * 0.4);
        let mut equivalent = with_ttiq.clone();
        equivalent.mitigations.ttiq.enabled = false;
        equivalent.infectious_period *= factor;
        equivalent.r0 *= factor;

        let actual = SEIRModel::new(with_ttiq).integrate(400);
        let expected = SEIRModel::new(equivalent).integrate(400);
        assert_outputs_close(&actual, &expected, 1e-8, 1e-6);
    }

    #[test]
    fn test_outcome_delays_preserve_totals() {
        let mut short = one_group_params();
        short.hospitalization_delay = 1.0;
        short.death_delay = 1.0;
        let mut long = short.clone();
        long.hospitalization_delay = 20.0;
        long.death_delay = 30.0;
        let short_output = SEIRModel::new(short).integrate(600);
        let long_output = SEIRModel::new(long).integrate(600);
        for output_type in [OutputType::HospitalIncidence, OutputType::DeathIncidence] {
            let a = total(&short_output, output_type.clone());
            let b = total(&long_output, output_type);
            assert!((a - b).abs() <= 1e-8_f64.max(1e-6 * a.abs()));
        }
    }

    #[test]
    fn test_population_scale_invariance() {
        let mut small = one_group_params();
        small.population = 1_000_000.0;
        small.initial_infections = 100.0;
        small.mitigations.vaccine.enabled = true;
        small.mitigations.vaccine.start = 0.0;
        small.mitigations.vaccine.ramp_up = 0.0;
        small.mitigations.vaccine.administration_rate = 10_000.0;
        small.mitigations.vaccine.doses_available = 500_000.0;
        let mut large = small.clone();
        large.population *= 10.0;
        large.initial_infections *= 10.0;
        large.mitigations.vaccine.administration_rate *= 10.0;
        large.mitigations.vaccine.doses_available *= 10.0;
        let small_model = SEIRModel::new(small);
        let large_model = SEIRModel::new(large);
        let small_state = vec![
            800_000.0, 20_000.0, 10_000.0, 50_000.0, 50_000.0, 5_000.0, 2_000.0, 10_000.0,
            30_000.0, 5_000.0, 3_000.0, 15_000.0,
        ];
        let large_state: Vec<f64> = small_state.iter().map(|value| value * 10.0).collect();

        for time in [10.0, 100.0] {
            let small_derivative = small_model.living_derivative_at(time, &small_state);
            let large_derivative = large_model.living_derivative_at(time, &large_state);
            for (small_value, large_value) in small_derivative.iter().zip(large_derivative) {
                let normalized_large = large_value / 10.0;
                let tolerance = 2e-12_f64.max(1e-10 * small_value.abs());
                assert!(
                    (normalized_large - small_value).abs() <= tolerance,
                    "scale mismatch at t={time}: small={small_value} large/10={normalized_large} tol={tolerance}"
                );
            }
        }
    }

    #[test]
    fn test_population_group_permutation() {
        let mut original = default_typed2();
        original.mitigations.vaccine.enabled = false;
        original.mitigations.antivirals.enabled = false;
        original.mitigations.community.enabled = false;
        original.mitigations.ttiq.enabled = false;
        let mut permuted = original.clone();
        permuted.population_fractions = Vector2::new(
            original.population_fractions[1],
            original.population_fractions[0],
        );
        permuted.population_fraction_labels = nalgebra::SVector::from([
            original.population_fraction_labels[1].clone(),
            original.population_fraction_labels[0].clone(),
        ]);
        permuted.contact_matrix = matrix![
            original.contact_matrix[(1, 1)], original.contact_matrix[(1, 0)];
            original.contact_matrix[(0, 1)], original.contact_matrix[(0, 0)]
        ];
        permuted.fraction_symptomatic = Vector2::new(
            original.fraction_symptomatic[1],
            original.fraction_symptomatic[0],
        );
        permuted.fraction_hospitalized = Vector2::new(
            original.fraction_hospitalized[1],
            original.fraction_hospitalized[0],
        );
        permuted.fraction_dead = Vector2::new(original.fraction_dead[1], original.fraction_dead[0]);

        let population = original.population;
        let original_output = SEIRModel::new(original).integrate(300);
        let permuted_output = SEIRModel::new(permuted).integrate(300);
        for output_type in [
            OutputType::InfectionIncidence,
            OutputType::SymptomaticIncidence,
            OutputType::HospitalIncidence,
            OutputType::DeathIncidence,
        ] {
            for (a, b) in original_output
                .get_output(&output_type)
                .iter()
                .zip(permuted_output.get_output(&output_type))
            {
                for (left, right) in [
                    (a.grouped_values[0], b.grouped_values[1]),
                    (a.grouped_values[1], b.grouped_values[0]),
                ] {
                    let tolerance = (1e-8 * population).max(1e-6 * left.abs());
                    assert!((left - right).abs() <= tolerance);
                }
            }
        }
    }
}
