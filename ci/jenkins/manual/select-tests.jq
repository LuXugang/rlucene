# Input order: ordinary inventory, then the requested feature's inventory (-s).
# Only feature-added tests belong to this job, regardless of ignore reason.
# Match binary IDs and full names, never substrings shared by different tests.
def test_rows:
  .["rust-suites"] | to_entries[] |
  .key as $binary | .value.testcases | to_entries[] |
  {binary: $binary, name: .key};
[.[0] | test_rows] as $ordinary |
([.[1] | test_rows] - $ordinary) |
map("(binary_id(=" + .binary + ") & test(=" + .name + "))") |
if length == 0 then "none()" else join(" | ") end
