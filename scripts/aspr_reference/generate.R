args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 2) {
  stop("usage: Rscript generate.R ASPR_SOURCE OUTPUT_JSON")
}

aspr_source <- normalizePath(args[[1]])
output_path <- args[[2]]
aspr_revision <- "1bee5a596c22a6387a56aa337be5741ca41117a8"

for (file in list.files(file.path(aspr_source, "R"), pattern = "[.]R$", full.names = TRUE)) {
  source(file)
}

days <- 60
n <- 2
population_fractions <- c(0.4, 0.6)
contact_matrix <- matrix(c(1.5, 0.2, 0.4, 1.0), nrow = n, byrow = TRUE)
community_effectiveness <- matrix(c(0.3, 0.1, 0.1, 0.2), nrow = n, byrow = TRUE)

pad_living <- function(values, doses) {
  living_count <- switch(as.character(doses), "0" = 4 * n, "1" = 8 * n, "2" = 12 * n)
  c(as.numeric(values[seq_len(living_count)]), rep(0, 12 * n - living_count))
}

run_scenario <- function(doses, antivirals, community) {
  scenario_days <- if (doses > 0) 5 else days
  common <- list(
    population = 1,
    populationFractions = population_fractions,
    contactMatrix = contact_matrix,
    R0 = 1.6,
    latentPeriod = 2,
    infectiousPeriod = 3,
    seedInfections = 0.01,
    priorImmunity = 0.1,
    useCommunityMitigation = community,
    simulationLength = scenario_days,
    seedStartDay = 0,
    tolerance = 1e-8,
    method = "lsoda"
  )
  if (community) {
    common$communityMitigationStartDay <- 10
    common$communityMitigationDuration <- 15
    common$communityMitigationMultiplier <- 1 - community_effectiveness
  }

  av <- list(
    fractionSymptomatic = c(0.5, 0.5),
    fractionSeekCare = 0.6,
    fractionDiagnosedAndPrescribedOutpatient = 0.7,
    fractionAdhere = 0.8,
    fractionAdmitted = 1,
    fractionDiagnosedAndPrescribedInpatient = 1,
    AVEi = 0.5,
    AVEp = 0
  )
  vaccine <- list(
    vaccineAdministrationRatePerDay = 0.02,
    vaccineAvailabilityByDay = c(0.4, rep(0, days - 1)),
    vaccineUptakeMultiplier = c(1, 1),
    vaccineEfficacyDelay = 5
  )

  if (doses == 0 && !antivirals) {
    model <- do.call(SEIRModel, common)
    derivative_function <- getDerivative.SEIR
  } else if (doses == 0) {
    model <- do.call(SEIRTModel, c(common, av))
    derivative_function <- getDerivative.SEIRT
  } else if (doses == 1 && !antivirals) {
    model <- do.call(SEIRVModel, c(common, vaccine, list(VEs = 0.4, VEi = 0.2, VEp = 0.3)))
    derivative_function <- getDerivative.SEIRV
  } else if (doses == 1) {
    model <- do.call(SEIRTVModel, c(common, av, vaccine, list(VEs = 0.4, VEi = 0.2, VEp = 0.3)))
    derivative_function <- getDerivative.SEIRTV
  } else {
    vaccine2 <- c(vaccine, list(
      dose2Delay = 7,
      VEs1 = 0.4, VEs2 = 0.7,
      VEi1 = 0.2, VEi2 = 0.4,
      VEp1 = 0.3, VEp2 = 0.6
    ))
    if (antivirals) {
      model <- do.call(SEIRTV2DoseModel, c(common, av, vaccine2))
      derivative_function <- getDerivative.SEIRTV2Dose
    } else {
      model <- do.call(SEIRV2DoseModel, c(common, vaccine2))
      derivative_function <- getDerivative.SEIRV2Dose
    }
  }

  raw <- model$rawOutput
  # ASPR counts doses administered to exposed/infectious/removed people in an
  # auxiliary V denominator, while cfa-flu-simulator moves only susceptible
  # people and derives its denominator from living compartments. Compare
  # vaccine scenarios before administration begins; their post-start behavior
  # is intentionally not asserted equivalent.
  derivative_time <- if (doses == 0) 12 else 4
  derivative_row <- which(abs(raw[, "time"] - derivative_time) < 1e-12)[[1]]
  full_state <- as.numeric(raw[derivative_row, -1])
  derivative <- derivative_function(derivative_time, full_state, model$parameters)[[1]]

  list(
    name = sprintf("v%s_av%s_community%s", doses, as.integer(antivirals), as.integer(community)),
    doses = doses,
    antivirals = antivirals,
    community = community,
    days = scenario_days,
    initial_living = pad_living(raw[1, -1], doses),
    trajectory = lapply(seq_len(nrow(raw)), function(index) {
      list(time = unname(raw[index, "time"]), living = pad_living(raw[index, -1], doses))
    }),
    derivative = list(
      time = derivative_time,
      living = pad_living(full_state, doses),
      expected = pad_living(derivative, doses)
    )
  )
}

scenarios <- list()
for (doses in 0:2) {
  for (antivirals in c(FALSE, TRUE)) {
    for (community in c(FALSE, TRUE)) {
      scenarios[[length(scenarios) + 1]] <- run_scenario(doses, antivirals, community)
    }
  }
}

fixture <- list(
  manifest = list(
    source_repository = "https://github.com/HHS/ASPR-flumodels",
    source_revision = aspr_revision,
    r_version = R.version.string,
    deSolve_version = as.character(utils::packageVersion("deSolve")),
    aspr_solver_absolute_tolerance = 1e-8
  ),
  scenarios = scenarios
)

dir.create(dirname(output_path), recursive = TRUE, showWarnings = FALSE)
jsonlite::write_json(fixture, output_path, auto_unbox = TRUE, digits = 16, pretty = TRUE)
