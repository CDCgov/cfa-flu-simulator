# ASPR-flumodels reference comparison verification

Scott Olesen <ulp7@cdc.gov> \| 2026-09-03

## Recommended actions

1. Initialize infections into $E$ and $I$ like ASPR-flumodels does
1. Create snapshot tests as of the current model state
1. Discard reference comparisons to dynode-web

## Model comparison

I spent some time verifying cfa-flu-simulator against ASPR-flumodels. I
concluded that the models differ because of three factors, described below. When
these three factors were controlled for, the outputs of the model were tolerably
identical for the convenience set of simulations, similar to the ones encoded in
the current regression tests.

### Infection seeding

ASPR-flumodels seeds infections into $E$ and $I$ according to the mean dwell
times
([code](https://github.com/HHS/ASPR-flumodels/blob/1bee5a596c22a6387a56aa337be5741ca41117a8/R/SEIRModel.R#L214)).
For mean incubation period $1/\lambda$, mean infectious period $1/\gamma$, and
initial infections $J$, the initial number exposed is $E_0 = (1/\lambda) J$ and
the initial number infectious is $I_0 = (1/\gamma) J$. Noting that
$E_0 + I_0 = J$, this implies:

$$
  \begin{align*}
    E_0 & = \frac{J_0}{1 + \lambda/\gamma} \\
    I_0 & = \frac{J_0}{1 + \gamma/\lambda}
  \end{align*}
$$

In the prior implementation of cfa-flu-simulator, all infections were seeded
into $I$. Seeding across $E$ and $I$ will produce slightly more intuitive early
infection dynamics.

### Outcome calculaation

ASPR-flumodels reports total (i.e., over the simulation) infections, symptomatic
infections (i.e., cases), hospitalizations, and deaths using a *post hoc*
method. It measures the number of total infections as the difference between the
size of the $R$ compartment at the start and end of the simulation
([code](https://github.com/HHS/ASPR-flumodels/blob/1bee5a596c22a6387a56aa337be5741ca41117a8/R/getInfections.R#L48)).
This excludes those individuals who are in the $I$ compartments at the end of
the simulation. Next, it multiplies the total infections by the relevant ratio
to produce total cases
([code](https://github.com/HHS/ASPR-flumodels/blob/1bee5a596c22a6387a56aa337be5741ca41117a8/R/getInfections.R#L39)),
hospitalizations
([code](https://github.com/HHS/ASPR-flumodels/blob/1bee5a596c22a6387a56aa337be5741ca41117a8/R/getHospitalizations.R#L39)),
and deaths
([code](https://github.com/HHS/ASPR-flumodels/blob/1bee5a596c22a6387a56aa337be5741ca41117a8/R/getDeaths.R#L39)).
Note that this limits ASPR-flumodels to vaccine effectiveness against
progression from infection to symptomatic infection; this framework does not
permit a separate effectiveness against progression from symptomatic infection
to hospitalization or from hospitalization to death.

cfa-flu-simulator computes hospitalizations and deaths as incident quantities at
the time of the $E \to I$ transition, which avoids truncating those in the $I$
compartment at the end of the simulation. This framework also permits multiple
types of vaccine effectiveness.

### Solvers

The codebases used different ODE solvers. Tweaking the exact solver used or the
step size had minimal effects, in line with expectations for those kinds of
tweaks. The differences between solvers were most marked in simulations that
included community mitigations and were most marked at the moment of the
mitigation onset. This is not surprising, because community mitigation is
modeled as a discontinuous change in the ODE rates, which different solvers will
respond to somewhat differently.

Overall, I found no evidence to suggest that the choice of solver is tightly
coupled with model design.
