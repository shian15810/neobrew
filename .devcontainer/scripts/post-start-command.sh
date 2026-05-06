#!/bin/sh

set -euvx

export LC_ALL="C"

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd -P)"

cd -- "${SCRIPT_DIR}/../.."

SCCACHE_SERVER_UDS="${SCCACHE_SERVER_UDS:?}"
SCCACHE_IDLE_TIMEOUT="${SCCACHE_IDLE_TIMEOUT:?}"

sccache --start-server

sccache --show-stats
