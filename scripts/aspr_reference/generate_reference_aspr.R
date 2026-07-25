# install the relevant repo and load it
repo <- "HHS/ASPR-flumodels"
repo_ref <- "1bee5a596c22a6387a56aa337be5741ca41117a8"

pak::pak(paste0(repo, "@", repo_ref), upgrade = FALSE)
library(flumodels)

# set up the output path -- this is relative to repo root
output_path <- "model/tests/reference_aspr.json"
verbose <- TRUE

# Baseline ASPR-flumodels execution settings that are not part of the Rust
# Parameters struct. These are varied independently below.
default_aspr_parameters <- list(
  seed_start_day = 0,
  solver_method = "lsoda",
  solver_absolute_tolerance = 1e-8,
  vaccine_availability_horizon_days = 60L,
  vaccine_uptake_multiplier = c(1, 1)
)

# names in the Parameters struct in model/src/parameters.rs
parameter_names <- c(
  "n",
  "days",
  "population",
  "population_fraction_labels",
  "population_fractions",
  "contact_matrix",
  "initial_infections",
  "fraction_initial_immune",
  "r0",
  "latent_period",
  "infectious_period",
  "fraction_symptomatic",
  "fraction_hospitalized",
  "hospitalization_delay",
  "fraction_dead",
  "death_delay",
  "p_test_sympto",
  "test_sensitivity",
  "p_test_forward",
  "vaccine_enabled",
  "vaccine_editable",
  "vaccine_doses",
  "vaccine_start",
  "vaccine_dose2_delay",
  "vaccine_p_get_2_doses",
  "vaccine_administration_rate",
  "vaccine_doses_available",
  "vaccine_ramp_up",
  "vaccine_ve_s",
  "vaccine_ve_i",
  "vaccine_ve_p",
  "vaccine_ve_2s",
  "vaccine_ve_2i",
  "vaccine_ve_2p",
  "antivirals_enabled",
  "antivirals_editable",
  "antivirals_fraction_adhere",
  "antivirals_fraction_diagnosed_prescribed_inpatient",
  "antivirals_fraction_diagnosed_prescribed_outpatient",
  "antivirals_fraction_seek_care",
  "antivirals_ave_i",
  "antivirals_ave_p_hosp",
  "antivirals_ave_p_death",
  "community_enabled",
  "community_editable",
  "community_start",
  "community_duration",
  "community_effectiveness",
  "ttiq_enabled",
  "ttiq_editable",
  "ttiq_p_id_infectious",
  "ttiq_p_infectious_isolates",
  "ttiq_isolation_reduction",
  "ttiq_p_contact_trace",
  "ttiq_p_traced_quarantines"
)

# Baseline Rust Parameters values shared by every point in the mitigation grid.
default_rust_parameters <- list(
  n = 2L,
  days = 100L,
  population = 1,
  population_fraction_labels = c("Group 1", "Group 2"),
  population_fractions = c(0.4, 0.6),
  # Column-major flat form used by R matrices and nalgebra.
  contact_matrix = c(1.5, 0.4, 0.2, 1),
  initial_infections = 0.01,
  fraction_initial_immune = 0.1,
  r0 = 1.6,
  latent_period = 2,
  infectious_period = 3,
  fraction_symptomatic = c(0.5, 0.5),
  fraction_hospitalized = c(0.1, 0.2),
  hospitalization_delay = 1,
  fraction_dead = c(0.01, 0.02),
  death_delay = 1,
  p_test_sympto = 0,
  test_sensitivity = 0.9,
  p_test_forward = 0.9,
  vaccine_enabled = FALSE,
  vaccine_editable = TRUE,
  vaccine_doses = 1L,
  vaccine_start = 0,
  vaccine_dose2_delay = 7,
  vaccine_p_get_2_doses = 1,
  vaccine_administration_rate = 0.02,
  vaccine_doses_available = 0.4,
  vaccine_ramp_up = 5,
  vaccine_ve_s = 0.4,
  vaccine_ve_i = 0.2,
  vaccine_ve_p = 0.3,
  vaccine_ve_2s = 0.7,
  vaccine_ve_2i = 0.4,
  vaccine_ve_2p = 0.6,
  antivirals_enabled = FALSE,
  antivirals_editable = TRUE,
  antivirals_fraction_adhere = 0.8,
  antivirals_fraction_diagnosed_prescribed_inpatient = 0.9,
  antivirals_fraction_diagnosed_prescribed_outpatient = 0.7,
  antivirals_fraction_seek_care = 0.6,
  antivirals_ave_i = 0.5,
  antivirals_ave_p_hosp = 0,
  antivirals_ave_p_death = 0,
  community_enabled = FALSE,
  community_editable = TRUE,
  community_start = 10,
  community_duration = 15,
  community_effectiveness = c(0.3, 0.1, 0.1, 0.2),
  ttiq_enabled = FALSE,
  ttiq_editable = TRUE,
  ttiq_p_id_infectious = 0,
  ttiq_p_infectious_isolates = 0,
  ttiq_isolation_reduction = 0,
  ttiq_p_contact_trace = 0,
  ttiq_p_traced_quarantines = 0
)

stopifnot(
  identical(names(default_rust_parameters), parameter_names),
  !any(vapply(default_rust_parameters, is.list, logical(1)))
)

# Generate the 3 x 2 x 2 mitigation grid as Rust Parameters-shaped lists.
generate_rust_parameter_sets <- function(default_parameters) {
  grid <- expand.grid(
    community_enabled = c(FALSE, TRUE),
    antivirals_enabled = c(FALSE, TRUE),
    vaccine_doses = 0:2,
    KEEP.OUT.ATTRS = FALSE
  )

  parameter_sets <- lapply(seq_len(nrow(grid)), function(index) {
    doses <- as.integer(grid$vaccine_doses[[index]])
    parameters <- default_parameters
    parameters$vaccine_enabled <- doses > 0L
    # Parameters requires a valid dose count even when vaccination is off.
    parameters$vaccine_doses <- max(doses, 1L)
    parameters$antivirals_enabled <- grid$antivirals_enabled[[index]]
    parameters$community_enabled <- grid$community_enabled[[index]]
    parameters
  })

  parameter_sets
}

# Generate ASPR-only parameter sets independently of the Rust parameter grid.
# Keeping this separate makes it explicit which settings are unavailable in
# the Rust model and allows more ASPR execution settings to be varied later.
generate_aspr_parameter_sets <- function(default_parameters) {
  grid <- expand.grid(
    solver_method = c("lsoda", "rk4"),
    KEEP.OUT.ATTRS = FALSE,
    stringsAsFactors = FALSE
  )

  lapply(seq_len(nrow(grid)), function(index) {
    parameters <- default_parameters
    parameters$solver_method <- grid$solver_method[[index]]
    parameters
  })
}

# Take the Cartesian product of the Rust and ASPR parameter domains. Each run
# retains both inputs so its output is fully attributable.
cross_parameter_sets <- function(rust_parameter_sets, aspr_parameter_sets) {
  do.call(
    c,
    lapply(rust_parameter_sets, function(rust_parameters) {
      lapply(aspr_parameter_sets, function(aspr_parameters) {
        list(
          rust_parameters = rust_parameters,
          aspr_parameters = aspr_parameters
        )
      })
    })
  )
}

manifest <- list(
  repo = repo,
  repo_ref = repo_ref,
  r_version = R.version.string,
  desolve_version = as.character(utils::packageVersion("deSolve"))
)

# Run one joint parameter set through the corresponding ASPR model.
run <- function(parameter_set) {
  rust_parameters <- parameter_set$rust_parameters
  aspr_parameters <- parameter_set$aspr_parameters
  n <- rust_parameters$n

  if (!rust_parameters$vaccine_enabled && !rust_parameters$antivirals_enabled) {
    model_function <- SEIRModel
  } else if (
    rust_parameters$vaccine_enabled &&
      rust_parameters$vaccine_doses == 1L &&
      !rust_parameters$antivirals_enabled
  ) {
    model_function <- SEIRVModel
  } else if (
    rust_parameters$vaccine_enabled &&
      rust_parameters$vaccine_doses == 2L &&
      !rust_parameters$antivirals_enabled
  ) {
    model_function <- SEIRV2DoseModel
  } else if (!rust_parameters$vaccine_enabled && rust_parameters$antivirals_enabled) {
    model_function <- SEIRTModel
  } else if (
    rust_parameters$vaccine_enabled &&
      rust_parameters$vaccine_doses == 1L &&
      rust_parameters$antivirals_enabled
  ) {
    model_function <- SEIRTVModel
  } else if (
    rust_parameters$vaccine_enabled &&
      rust_parameters$vaccine_doses == 2L &&
      rust_parameters$antivirals_enabled
  ) {
    model_function <- SEIRTV2DoseModel
  } else {
    stop("Unsupported parameterization")
  }

  model_parameters <- list(
    population = rust_parameters$population,
    populationFractions = rust_parameters$population_fractions,
    contactMatrix = matrix(rust_parameters$contact_matrix, nrow = n, ncol = n),
    R0 = rust_parameters$r0,
    latentPeriod = rust_parameters$latent_period,
    infectiousPeriod = rust_parameters$infectious_period,
    seedInfections = rust_parameters$initial_infections,
    priorImmunity = rust_parameters$fraction_initial_immune,
    useCommunityMitigation = rust_parameters$community_enabled,
    simulationLength = rust_parameters$days,
    seedStartDay = aspr_parameters$seed_start_day,
    tolerance = aspr_parameters$solver_absolute_tolerance,
    method = aspr_parameters$solver_method
  )

  if (rust_parameters$community_enabled) {
    effectiveness <- matrix(
      rust_parameters$community_effectiveness,
      nrow = n,
      ncol = n
    )
    model_parameters <- c(
      model_parameters,
      list(
        communityMitigationStartDay = rust_parameters$community_start,
        communityMitigationDuration = rust_parameters$community_duration,
        communityMitigationMultiplier = 1 - effectiveness
      )
    )
  }

  if (rust_parameters$antivirals_enabled) {
    model_parameters <- c(
      model_parameters,
      list(
        fractionSymptomatic = rust_parameters$fraction_symptomatic,
        fractionSeekCare = rust_parameters$antivirals_fraction_seek_care,
        fractionDiagnosedAndPrescribedOutpatient = rust_parameters$antivirals_fraction_diagnosed_prescribed_outpatient,
        fractionAdhere = rust_parameters$antivirals_fraction_adhere,
        fractionAdmitted = rust_parameters$fraction_hospitalized,
        fractionDiagnosedAndPrescribedInpatient = rust_parameters$antivirals_fraction_diagnosed_prescribed_inpatient,
        AVEi = rust_parameters$antivirals_ave_i,
        AVEp = rust_parameters$antivirals_ave_p_hosp
      )
    )
  }

  if (rust_parameters$vaccine_enabled) {
    availability_horizon <- max(
      rust_parameters$days,
      aspr_parameters$vaccine_availability_horizon_days,
      as.integer(rust_parameters$vaccine_start) + 1L
    )
    vaccine_availability <- rep(0, availability_horizon)
    vaccine_availability[as.integer(rust_parameters$vaccine_start) + 1L] <-
      rust_parameters$vaccine_doses_available

    model_parameters <- c(
      model_parameters,
      list(
        vaccineAdministrationRatePerDay = rust_parameters$vaccine_administration_rate,
        vaccineAvailabilityByDay = vaccine_availability,
        vaccineUptakeMultiplier = aspr_parameters$vaccine_uptake_multiplier,
        vaccineEfficacyDelay = rust_parameters$vaccine_ramp_up
      )
    )

    if (rust_parameters$vaccine_doses == 1L) {
      model_parameters <- c(
        model_parameters,
        list(
          VEs = rust_parameters$vaccine_ve_s,
          VEi = rust_parameters$vaccine_ve_i,
          VEp = rust_parameters$vaccine_ve_p
        )
      )
    } else if (rust_parameters$vaccine_doses == 2L) {
      model_parameters <- c(
        model_parameters,
        list(
          dose2Delay = rust_parameters$vaccine_dose2_delay,
          VEs1 = rust_parameters$vaccine_ve_s,
          VEs2 = rust_parameters$vaccine_ve_2s,
          VEi1 = rust_parameters$vaccine_ve_i,
          VEi2 = rust_parameters$vaccine_ve_2i,
          VEp1 = rust_parameters$vaccine_ve_p,
          VEp2 = rust_parameters$vaccine_ve_2p
        )
      )
    } else {
      stop("ASPR-flumodels supports only one- and two-dose vaccines")
    }
  }

  # run the model
  model <- do.call(model_function, model_parameters)

  # extract key data
  times <- with(
    model_parameters,
    seq(seedStartDay, seedStartDay + simulationLength, by = 1)
  )

  caseFatalityRatio <- rust_parameters$fraction_dead /
    rust_parameters$fraction_symptomatic
  caseHospitalizationRatio <- rust_parameters$fraction_hospitalized /
    rust_parameters$fraction_symptomatic
  if (rust_parameters$antivirals_enabled) {
    infections <- getInfectionTimeSeries(model)
    total_deaths <- getDeaths(model, caseFatalityRatio = caseFatalityRatio)
    total_hosps <- getHospitalizations(
      model,
      caseHospitalizationRatio = caseHospitalizationRatio
    )
  } else {
    fractionSymptomatic <- rust_parameters$fraction_symptomatic
    infections <- getInfectionTimeSeries(
      model,
      fractionSymptomatic = fractionSymptomatic
    )
    total_deaths <- getDeaths(
      model,
      fractionSymptomatic = fractionSymptomatic,
      caseFatalityRatio = caseFatalityRatio
    )
    total_hosps <- getHospitalizations(
      model,
      fractionSymptomatic = fractionSymptomatic,
      caseHospitalizationRatio = caseHospitalizationRatio
    )
  }

  stopifnot(length(times) == dim(infections)[1])

  list(
    manifest = manifest,
    rust_parameters = rust_parameters,
    aspr_parameters = aspr_parameters,
    output = list(
      times = times,
      infections = infections,
      total_deaths = total_deaths,
      total_hosps = total_hosps
    )
  )
}

rust_parameter_sets <- generate_rust_parameter_sets(default_rust_parameters)
aspr_parameter_sets <- generate_aspr_parameter_sets(default_aspr_parameters)
parameter_sets <- cross_parameter_sets(rust_parameter_sets, aspr_parameter_sets)
stopifnot(
  length(parameter_sets) ==
    length(rust_parameter_sets) * length(aspr_parameter_sets)
)
results <- unname(lapply(parameter_sets, run))

jsonlite::write_json(
  results,
  output_path,
  auto_unbox = TRUE,
  digits = 12,
  pretty = TRUE
)
