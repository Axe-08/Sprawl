#!/bin/bash
exec > /home/akshit/Projects/Sprawl/test_out.txt 2>&1
echo "=== STEP 0 ==="
ls -la /home/akshit/Projects/Sprawl/target/release/sprawl-cli
if [ ! -f /home/akshit/Projects/Sprawl/target/release/sprawl-cli ]; then
  cd /home/akshit/Projects/Sprawl && cargo build --release 2>&1 | tail -20
fi

echo "=== STEP 1 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli --help

echo "=== STEP 2 ==="
for cmd in status daemon analyze scan triage profile-machine search index plugin bundle resurrect restore simulate-revoke verify; do
  echo "--- $cmd ---"
  /home/akshit/Projects/Sprawl/target/release/sprawl-cli $cmd --help 2>&1 | head -5
done

echo "=== STEP 3 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli status

echo "=== STEP 4 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli status --json

echo "=== STEP 5 ==="
pkill -f sprawl-cli 2>/dev/null; sleep 1
/home/akshit/Projects/Sprawl/target/release/sprawl-cli daemon status
echo "daemon-status-exitcode: $?"

echo "=== STEP 6 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli daemon start &
sleep 3
/home/akshit/Projects/Sprawl/target/release/sprawl-cli daemon status
echo "daemon-running-exitcode: $?"

echo "=== STEP 7 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli analyze /home/akshit/Projects/Sprawl
sqlite3 ~/.sprawl/ledger.sqlite "SELECT root_path, ecosystem, status FROM projects;" 2>/dev/null || echo 'ledger empty or missing'

echo "=== STEP 8 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli analyze /home/akshit/Projects/Sprawl --json

echo "=== STEP 9 ==="
echo 'AKIAIOSFODNN7EXAMPLE' > /tmp/test_secret.txt
echo 'GHTOKEN_ghp_abcXYZ1234567890RANDOM_extra_entropy_pad' >> /tmp/test_secret.txt
/home/akshit/Projects/Sprawl/target/release/sprawl-cli scan /tmp/test_secret.txt

echo "=== STEP 10 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli scan /tmp/test_secret.txt --json

echo "=== STEP 11 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli triage list

echo "=== STEP 12 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli triage list --json

echo "=== STEP 13 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli triage nuke /tmp/fake_nonexistent_dir
echo "triage-nuke-invalid-exitcode: $?"

echo "=== STEP 14 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli triage snooze /tmp/fake_nonexistent_dir
echo "triage-snooze-invalid-exitcode: $?"

echo "=== STEP 15 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli profile-machine

echo "=== STEP 16 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli profile-machine --json

echo "=== STEP 17 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli search "tokio async"

echo "=== STEP 18 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli index --start

echo "=== STEP 19 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli plugin list

echo "=== STEP 20 ==="
ls /home/akshit/Projects/Sprawl/plugins/ 2>/dev/null | head -5
/home/akshit/Projects/Sprawl/target/release/sprawl-cli plugin verify --help

echo "=== STEP 21 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli bundle /home/akshit/Projects/Sprawl --output /tmp/sprawl_test_bundle.tar.gz
echo "bundle-exitcode: $?"

echo "=== STEP 22 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli resurrect 00000000-0000-0000-0000-000000000000
echo "resurrect-nonexistent-exitcode: $?"

echo "=== STEP 23 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli restore /tmp/totally_fake_path_xyz
echo "restore-nonexistent-exitcode: $?"

echo "=== STEP 24 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli simulate-revoke fake-key-12345
echo "simulate-revoke-exitcode: $?"

echo "=== STEP 25 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli verify --help

echo "=== STEP 26 ==="
sleep 5
/home/akshit/Projects/Sprawl/target/release/sprawl-cli status

echo "=== STEP 27 ==="
pkill -f sprawl-cli 2>/dev/null; sleep 1
/home/akshit/Projects/Sprawl/target/release/sprawl-cli daemon status
echo "stopped-daemon-exitcode: $?"

echo "=== STEP 28 ==="
/home/akshit/Projects/Sprawl/target/release/sprawl-cli status --json

rm -f /tmp/test_secret.txt /tmp/sprawl_test_bundle.tar.gz
