#!/bin/sh

set -euvx

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd)"

SCCACHE_SERVER_UDS="${SCCACHE_SERVER_UDS:?}"
SCCACHE_IDLE_TIMEOUT="${SCCACHE_IDLE_TIMEOUT:?}"

cd -- "$SCRIPT_DIR/../.."

sccache --start-server

sccache --show-stats
