"""Cross-test isolation (QA-02): the history store singleton, Fernet cache,
audit resolver and rate-limit buckets are process-global in netrail. Without
explicit reset, a test that opened a store against its own NETRAIL_DB_PATH /
HOME leaves the singleton bound to stale settings, and the next test can hit
HISTORY_DISABLED or read another test's database.

The autouse fixture below resets every global before each test, making the
suite deterministic regardless of individual test hygiene (parity with the
`#[serial_test::serial]` + per-binary-process isolation convention used by the
Rust integration tests in src-tauri/tests).
"""

import pytest


@pytest.fixture(autouse=True)
def _reset_process_globals():
    from netrail.history import crypto as history_crypto
    from netrail.history import store as history_store
    from netrail.history.store import reset_store_for_tests
    from netrail import audit
    from netrail import rate_limit

    reset_store_for_tests()
    history_crypto.reset_for_tests()
    audit.reset_for_tests()
    rate_limit.set_test_limits()
    yield
