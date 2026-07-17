#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
REVISION=1bee5a596c22a6387a56aa337be5741ca41117a8
IMAGE=docker.io/rocker/r-ver:4.4.3
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

git clone https://github.com/HHS/ASPR-flumodels.git "$WORK/ASPR-flumodels"
git -C "$WORK/ASPR-flumodels" checkout "$REVISION"

podman run --rm \
  -v "$ROOT:/work:Z" \
  -v "$WORK/ASPR-flumodels:/aspr:ro,Z" \
  "$IMAGE" \
  bash -lc 'install2.r --error deSolve jsonlite && Rscript /work/scripts/aspr_reference/generate.R /aspr /work/model/tests/aspr_reference/aspr_reference.json'
