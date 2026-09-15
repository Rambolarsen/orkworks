#!/bin/sh
set -eu

if [ "$(uname -s)" != "Darwin" ]; then
    echo "launchd fixture is supported only on macOS" >&2
    exit 77
fi

if [ "$#" -ne 2 ]; then
    echo "usage: $0 LABEL PLIST" >&2
    exit 64
fi

label=$1
plist=$2
domain="gui/$(id -u)"
target="$domain/$label"

launchctl bootstrap "$domain" "$plist"
cleanup() {
    launchctl bootout "$target" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

launchctl print "$target"
