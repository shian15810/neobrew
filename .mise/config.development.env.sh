#!/bin/sh

set -eu

export LC_ALL="C"

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd -P)"

cd -- "${SCRIPT_DIR}/.."

if [ "$(uname)" = "Darwin" ]; then
    MDT="$(sw_vers -productVersion)"
    MDT="$(printf '%s' "$MDT" | cut -d'.' -f1-2)"

    export CARGO_PKG_METADATA_SCCACHE_MACOSX_DEPLOYMENT_TARGET="$MDT"

    export MACOSX_DEPLOYMENT_TARGET="$MDT"
fi

unset -v LC_ALL
