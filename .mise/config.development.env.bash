#!/usr/bin/env bash

set -Eeuo pipefail

shopt -s inherit_errexit xpg_echo

export LC_ALL="C"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

cd -- "${SCRIPT_DIR}/.."

RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:?}"

# RUSTFLAGS
# RUSTDOCFLAGS

set -- --cfg="tokio_unstable"

case "$RUSTUP_TOOLCHAIN" in
    "stable") set -- "$@" --allow="unused-crate-dependencies" ;;
    "beta") ;;
    "nightly") ;;
    *) ;;
esac

set -- "$@" --force-warn="rustdoc::missing-doc-code-examples"

case "$RUSTUP_TOOLCHAIN" in
    "stable") set -- "$@" --force-warn="linker-messages" ;;
    "beta") set -- "$@" --force-warn="linker-info" ;;
    "nightly") set -- "$@" --force-warn="linker-info" ;;
    *) ;;
esac

export RUSTFLAGS="$*"
export RUSTDOCFLAGS="$*"

# CLIPPYFLAGS

set -- --force-warn="dead-code"

set -- "$@" --force-warn="clippy::multiple-crate-versions"

case "$RUSTUP_TOOLCHAIN" in
    "stable") set -- "$@" --force-warn="linker-messages" ;;
    "beta") set -- "$@" --force-warn="linker-info" ;;
    "nightly") set -- "$@" --force-warn="linker-info" ;;
    *) ;;
esac

export CLIPPYFLAGS="$*"

# CARGO_SCCACHE_MACOSX_DEPLOYMENT_TARGET
# MACOSX_DEPLOYMENT_TARGET

if [[ "$(uname)" == "Darwin" ]]; then
    MACOSX_DEPLOYMENT_TARGET="$(sw_vers -productVersion)"
    MACOSX_DEPLOYMENT_TARGET="$(
        printf '%s' "$MACOSX_DEPLOYMENT_TARGET" | cut -d'.' -f1-2
    )"

    export CARGO_SCCACHE_MACOSX_DEPLOYMENT_TARGET="$MACOSX_DEPLOYMENT_TARGET"

    export MACOSX_DEPLOYMENT_TARGET
fi

unset -v LC_ALL
