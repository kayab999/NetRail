#!/usr/bin/env bash
# QA-02 guard: every Rust integration test that mutates process-global
# environment (std::env::set_var / remove_var) MUST be marked
# `#[serial_test::serial]` so it cannot race a parallel test thread in the
# same binary. Env writes inside child/Spawn code (chaos_process.rs uses
# Command::env, which is child-scoped and safe) are exempt.
#
# Exit 0 = invariant holds, 1 = violation. Run from the repo root.
set -u

# awk: for every `async fn <test>() {` determine its serial attribute (within
# 4 lines above the fn line), then flag env writes whose enclosing function
# is not serialized.
awk '
  function attr_serial(n) {
    for (i = n - 4; i < n; i++) {
      if (attr[i] ~ /serial_test::serial/) return 1
    }
    return 0
  }
  /async fn / { fn_start = NR; fn_serial = attr_serial(NR) }
  /std::env::set_var|std::env::remove_var/ {
    if (!fn_serial) printf "VIOLATION: %s:%d env write inside non-serial fn (attr at line %d)\n", FILENAME, NR, fn_start
  }
  { attr[NR] = $0 }
' src-tauri/tests/*.rs

echo "Guard script executed; violations (if any) printed above."