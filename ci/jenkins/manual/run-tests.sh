#!/usr/bin/env bash
set -euo pipefail

suite="${1:?nightly or monster is required}"
case "$suite" in
  nightly|monster) ;;
  *) printf 'Unknown heavy suite: %s\n' "$suite" >&2; exit 2 ;;
esac
: "${TMPDIR:?a build-specific temporary directory is required}"

# Feature-gated heavy tests are ignored by libtest. --run-ignored all alone
# would ALSO enable ordinary tests disabled for unrelated bugs. Compare the
# compiled inventories instead: select only tests added by this feature, not
# ordinary tests (whether ignored or not).
rm -f manual-tests.log manual-junit.xml manual-selection.txt manual-doctest.log
rm -f target/nextest/manual/junit.xml
exec > >(tee -a manual-tests.log) 2>&1
printf 'Suite: %s only; release; separate persistent cache: %s\n' \
  "$suite" "$CARGO_TARGET_DIR"

# Compile the requested feature first, making deployment smoke checks explicit.
cargo nextest list --release --workspace --features "$suite" \
  --profile manual --list-type binaries-only --message-format json > "$TMPDIR/feature-binaries.json"
cargo nextest list --release --workspace \
  --profile manual --message-format json > "$TMPDIR/ordinary-tests.json"
cargo metadata --format-version 1 --features "$suite" > "$TMPDIR/cargo-metadata.json"
cargo nextest list --profile manual --run-ignored all \
  --binaries-metadata "$TMPDIR/feature-binaries.json" \
  --cargo-metadata "$TMPDIR/cargo-metadata.json" \
  --message-format json > "$TMPDIR/feature-tests.json"

jq -sr -f ci/jenkins/manual/select-tests.jq \
  "$TMPDIR/ordinary-tests.json" "$TMPDIR/feature-tests.json" \
  > manual-selection.txt
filter="$(cat manual-selection.txt)"
if [ -z "$filter" ] || [ "$filter" = 'none()' ]; then
  printf 'No feature-exclusive tests were selected for %s\n' "$suite" >&2
  exit 1
fi
# Reuse the feature binaries even though the default inventory was built second.
test_environment=(env 'tests.light=false' 'tests.nightly=false')
if [ "$suite" = nightly ]; then
  test_environment=(env 'tests.light=false' 'tests.nightly=true')
fi

set +e
"${test_environment[@]}" cargo nextest run --profile manual --run-ignored all \
  --binaries-metadata "$TMPDIR/feature-binaries.json" \
  --cargo-metadata "$TMPDIR/cargo-metadata.json" \
  -E "$filter"
test_status=$?
set -e
if [ -f target/nextest/manual/junit.xml ]; then
  cp target/nextest/manual/junit.xml manual-junit.xml
fi
# Ordinary tests and doctests are covered by rlucene-ci, not these heavy jobs.
exit "$test_status"
