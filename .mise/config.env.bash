#!/usr/bin/env bash

set -Eeuo pipefail

shopt -s xpg_echo
shopt -s inherit_errexit || true

export LC_ALL="C"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

cd -- "${SCRIPT_DIR}/.."

RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:?}"

# CARGO_UNSTABLE_CARGO_LINTS

if [[ $RUSTUP_TOOLCHAIN == "nightly" ]]; then
    export CARGO_UNSTABLE_CARGO_LINTS=true
fi

unset -v LC_ALL
