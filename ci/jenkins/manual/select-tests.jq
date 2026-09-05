# Exclude only baseline ignored tests. Keeping the exclusion list (rather than
# enumerating thousands of ordinary tests) also stays under Linux's argv limit.
# Match binary IDs and full names, never substrings shared by different tests.
def test_rows:
  .["rust-suites"] | to_entries[] |
  .key as $binary | .value.testcases | to_entries[] |
  {binary: $binary, name: .key, ignored: .value.ignored};
[test_rows | select(.ignored) |
  "(binary_id(=" + .binary + ") & test(=" + .name + "))"] |
if length == 0 then "all()" else "not (" + join(" | ") + ")" end
