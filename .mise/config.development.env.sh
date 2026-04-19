#!/bin/sh

set -eu

export LC_ALL='C'

if [ "$(uname)" = "Darwin" ]; then
    MDT="$(sw_vers -productVersion)"
    MDT="$(printf '%s' "$MDT" | cut -d'.' -f1-2)"

    export MACOSX_DEPLOYMENT_TARGET="$MDT"

    export CARGO_PKG_METADATA_SCCACHE_MACOSX_DEPLOYMENT_TARGET="$MDT"
fi

unset -v LC_ALL
