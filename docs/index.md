# CFA Flu Simulator

The CFA Flu Simulator is a browser-based tool for exploring influenza scenarios.
It runs a SEIR compartmental model locally in the browser,
so parameters can be changed and the epidemic curve
redrawn without a round trip to a server.

::: warning
This project is in the early stages of development and is not ready for
production use.
:::

## What the model covers

The model tracks disease dynamics across age groups and layers public health
interventions on top of them:

- **Vaccination**, including single- and two-dose campaigns with a
  dose-to-protection delay and separate effectiveness against infection,
  onward transmission, and symptoms.
- **Antivirals**, both outpatient and inpatient, reducing transmission and
  progression to hospitalization and death.
- **Community mitigations**, which scale contact rates for a fixed window.
- **Testing, tracing, isolation, and quarantine (TTIQ)**, approximated as a
  reduction in the mean infectious period.

Hospitalizations and deaths are counted as outcomes of infection but are
assumed not to feed back into transmission.

For the compartments, parameters, and equations, see the
[model description](/model).

## How it is built

The model is written in Rust and compiled to WebAssembly, with a Vue and
cfa-simulator frontend.

## Running it locally

Tasks are run with [plz](https://plzplz.org):

```sh
pnpm install
plz dev
```

To work on these docs instead:

```sh
plz docs dev
```

## Notices

This repository is a work of the United States Government, released into the
public domain under [CC0 1.0][cc0], with source code under the
[Apache License 2.0][apache]. Use of the simulator is limited to non-sensitive,
publicly available data; see the [disclaimer][disclaimer] for details.

[cc0]: https://creativecommons.org/publicdomain/zero/1.0/
[apache]: http://www.apache.org/licenses/LICENSE-2.0.html
[disclaimer]: https://github.com/CDCgov/cfa-flu-simulator/blob/main/DISCLAIMER.md
