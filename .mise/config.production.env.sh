#!/bin/sh

set -eu

export LC_ALL="C"

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd -P)"

RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:?}"

cd -- "${SCRIPT_DIR}/.."

RUSTFLAGS="--cfg=tokio_unstable --deny=warnings"

case "$RUSTUP_TOOLCHAIN" in
    "stable") RUSTFLAGS="${RUSTFLAGS} --force-warn=linker-messages" ;;
    "beta" | "nightly") RUSTFLAGS="${RUSTFLAGS} --force-warn=linker-info" ;;
    *) ;;
esac

export RUSTFLAGS

if [ "$(uname)" = "Darwin" ]; then
    MDT="$(rustc +"$RUSTUP_TOOLCHAIN" --print="deployment-target")"
    MDT="$(printf '%s' "$MDT" | cut -d'=' -f2)"

    export CARGO_PKG_METADATA_SCCACHE_MACOSX_DEPLOYMENT_TARGET="$MDT"

    export MACOSX_DEPLOYMENT_TARGET="$MDT"
fi

unset -v LC_ALL
