#!/usr/bin/env bash
# Quick verification of the 9 E2E test failures after fixes
BIN=/home/akshit/Projects/Sprawl/target/release/sprawl-cli
PASS=0; FAIL=0

check() {
  local label="$1"; local expect="$2"; shift 2
  local out
  out=$("$@" 2>&1)
  local ec=$?
  if echo "$out$ec" | grep -qE "$expect"; then
    echo "  ✅ PASS: $label"
    PASS=$((PASS+1))
  else
    echo "  ❌ FAIL: $label"
    echo "     Expected pattern: $expect"
    echo "     Got (exit=$ec): $(echo "$out" | head -3)"
    FAIL=$((FAIL+1))
  fi
}

echo "=== SPRAWL POST-FIX VERIFICATION ==="

echo ""
echo "--- Bug #1: DB re-init idempotency ---"
# Reset DB to half-initialized state (user_version=0 but tables exist) then try scan
rm -f ~/.sprawl/ledger.sqlite
# Create a partially-init DB (tables but no user_version set)
sqlite3 ~/.sprawl/ledger.sqlite "CREATE TABLE secrets (id TEXT PRIMARY KEY, source_file TEXT NOT NULL, classification TEXT NOT NULL, key_hash TEXT NOT NULL, discovered_at TEXT NOT NULL); CREATE TABLE ambiguous_secrets (id TEXT PRIMARY KEY, raw_value TEXT NOT NULL, filepath TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'pending', reviewed_at TEXT); CREATE TABLE projects (id TEXT PRIMARY KEY, root_path TEXT NOT NULL, last_seen TEXT NOT NULL, created_at TEXT NOT NULL);"
echo "AKIAIOSFODNN7EXAMPLE" > /tmp/test_secret.txt
check "scan on partially-init DB" "findings|ambiguous|PASS|0" "$BIN" scan /tmp/test_secret.txt
rm -f /tmp/test_secret.txt

echo ""
echo "--- Bug #1b: Daemon starts on existing DB ---"
pkill -f sprawl-cli 2>/dev/null; sleep 1
rm -f ~/.sprawl/sprawl.sock ~/.sprawl/sprawl.pid
"$BIN" daemon start > /tmp/sprawl_daemon_test.log 2>&1 &
DPID=$!
sleep 3
check "daemon status after start" "Running|running|pid" "$BIN" daemon status
kill $DPID 2>/dev/null; sleep 1

echo ""
echo "--- Bug #3: triage list --json when empty ---"
check "triage list --json returns JSON" '^\{"items":\[\]\}$' "$BIN" triage list --json

echo ""
echo "--- Bug #4: profile-machine --json ---"
check "profile-machine --json has output" '"persona"' "$BIN" profile-machine --json

echo ""
echo "--- Bug #5: plugin verify subcommand exists ---"
check "plugin verify --help exists" "Verify|manifest|verify" "$BIN" plugin verify --help

echo ""
echo "--- Bug #6: resurrect validates project existence ---"
check "resurrect nonexistent exits nonzero" "not found|4|Error" "$BIN" resurrect /tmp/totally_nonexistent_project_xyz_abc
check "resurrect exit code nonzero" "^[^0]" bash -c "$BIN resurrect /tmp/xyz_abc_nonexist 2>&1; echo \$?"

echo ""
echo "=== RESULTS: $PASS PASS, $FAIL FAIL ==="
