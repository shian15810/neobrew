#!/bin/sh

set -eu

if [ "$(uname -s)" = "Darwin" ]; then
    MACOSX_DEPLOYMENT_TARGET="$(sw_vers -productVersion)"
    MACOSX_DEPLOYMENT_TARGET="$(printf '%s' "$MACOSX_DEPLOYMENT_TARGET" | cut -d'.' -f1,2)"

    export MACOSX_DEPLOYMENT_TARGET

    export CARGO_PKG_METADATA_SCCACHE_MACOSX_DEPLOYMENT_TARGET="$MACOSX_DEPLOYMENT_TARGET"
fi
