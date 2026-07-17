# Generate ASPR reference fixtures

Run `scripts/aspr_reference/generate.sh` from any directory. The script
checks out the recorded archived ASPR-flumodels revision, runs it in R 4.4.3,
and replaces `model/tests/aspr_reference/aspr_reference.json`. Review fixture and
manifest changes together. Ordinary model tests consume the committed fixture
and do not require R or Podman.
