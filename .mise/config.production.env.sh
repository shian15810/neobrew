#!/bin/sh

set -eu

export LC_ALL='C'

if [ "$(uname)" = "Darwin" ]; then
    MDT="$(rustc --print="deployment-target")"
    MDT="$(printf '%s' "$MDT" | cut -d'=' -f2)"

    export MACOSX_DEPLOYMENT_TARGET="$MDT"

    export CARGO_PKG_METADATA_SCCACHE_MACOSX_DEPLOYMENT_TARGET="$MDT"
fi

unset -v LC_ALL
