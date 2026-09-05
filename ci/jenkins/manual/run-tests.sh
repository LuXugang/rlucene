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
# compiled inventories instead: include every non-ignored test plus tests that
# only exist with this feature, preserving the baseline's ignored tests.
rm -f manual-tests.log manual-junit.xml manual-selection.txt manual-doctest.log
rm -f target/nextest/manual/junit.xml
exec > >(tee -a manual-tests.log) 2>&1
printf 'Suite: ordinary + %s; release; separate persistent cache: %s\n' \
  "$suite" "$CARGO_TARGET_DIR"

# Compile the requested feature first, making deployment smoke checks explicit.
cargo nextest list --release --workspace --features "$suite" \
  --profile manual --list-type binaries-only --message-format json > "$TMPDIR/feature-binaries.json"
cargo nextest list --release --workspace \
  --profile manual --message-format json > "$TMPDIR/ordinary-tests.json"
cargo metadata --format-version 1 --features "$suite" > "$TMPDIR/cargo-metadata.json"

jq -r -f ci/jenkins/manual/select-tests.jq "$TMPDIR/ordinary-tests.json" \
  > manual-selection.txt
filter="$(cat manual-selection.txt)"
if [ -z "$filter" ]; then
  printf 'No ordinary or heavy tests were selected\n' >&2
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
# Preserve ordinary doctest coverage like rlucene-ci, including when unit tests fail.
set +e
"${test_environment[@]}" cargo test --release --workspace --features "$suite" --doc -q \
  > manual-doctest.log 2>&1
doc_status=$?
set -e
cat manual-doctest.log
if [ "$test_status" -ne 0 ]; then exit "$test_status"; fi
exit "$doc_status"
