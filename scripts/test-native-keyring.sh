#!/bin/bash
# Real Secret Service backend, isolated from any user's desktop keyring.
set -euo pipefail
umask 077
keyring_test_root=$(mktemp -d)
export XDG_DATA_HOME="$keyring_test_root/data"
export XDG_CONFIG_HOME="$keyring_test_root/config"
export XDG_RUNTIME_DIR="$keyring_test_root/runtime"
mkdir -p "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_RUNTIME_DIR"
dbus-run-session -- bash -c '
set -euo pipefail
printf "%s" "ephemeral-ci-keyring" | gnome-keyring-daemon --unlock --components=secrets >/dev/null
export OPENCORDE_TEST_OS_KEYS=1
cargo test --test crypto_ipc --locked
'
