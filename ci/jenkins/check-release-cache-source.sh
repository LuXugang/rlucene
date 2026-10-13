#!/bin/bash
set -euo pipefail

: "${CARGO_TARGET_DIR:?CARGO_TARGET_DIR must identify the job release cache}"
test -f Cargo.toml
test -f src/lib.rs
source_root="$(pwd -P)"

# env! embeds this path in the test program. A target reused from another
# checkout can remain fresh even after that checkout has been removed.
shopt -s nullglob
dependency_files=("$CARGO_TARGET_DIR"/release/deps/rlucene-*.d)
for dependency_file in "${dependency_files[@]}"; do
  cached_source_root="$(sed -n \
    's/^# env-dep:CARGO_MANIFEST_DIR=//p' "$dependency_file")"
  if [ -n "$cached_source_root" ] &&
     [ "$cached_source_root" != "$source_root" ]; then
    printf 'Release cache contains a different source directory: %s\n' \
      "$cached_source_root"
    printf 'Current source directory: %s\n' "$source_root"
    printf 'Invalidating only rlucene release artifacts; retaining dependencies.\n'
    cargo clean --release -p rlucene
    exit 0
  fi
done

printf 'Release cache source check passed: %s\n' "$source_root"
