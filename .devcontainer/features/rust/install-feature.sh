#!/bin/sh

set -euvx

export LC_ALL="C"

RUSTUP_HOME="${RUSTUP_HOME:?}"
CARGO_HOME="${CARGO_HOME:?}"

RUSTUP_PERMIT_COPY_RENAME="${RUSTUP_PERMIT_COPY_RENAME:?}"

NIGHTLY_COMPONENTS="${NIGHTLY_COMPONENTS:?}"

NIGHTLY_TOOLCHAIN_INSTALLED="${NIGHTLY_TOOLCHAIN_INSTALLED:?}"

rustup self update

DEFAULT_TOOLCHAIN="$(rustup default)"
DEFAULT_TOOLCHAIN="$(printf '%s' "$DEFAULT_TOOLCHAIN" | awk '{ print $1 }')"

rustup toolchain link "system" -- \
    "${RUSTUP_HOME}/toolchains/${DEFAULT_TOOLCHAIN}"
rustup default "system"

rustup update

rustup toolchain install "$@"

if [ -n "$NIGHTLY_COMPONENTS" ]; then
    NIGHTLY_COMPONENTS="$(printf '%s' "$NIGHTLY_COMPONENTS" | tr ',' ' ')"

    if [ "$NIGHTLY_TOOLCHAIN_INSTALLED" = true ]; then
        rustup +nightly component add "$NIGHTLY_COMPONENTS"
    fi
fi

rm -rf -- \
    "${RUSTUP_HOME}/downloads" \
    "${RUSTUP_HOME}/tmp" \
    "${RUSTUP_HOME}/update-hashes"

rm -rf -- \
    "${CARGO_HOME}/git" \
    "${CARGO_HOME}/registry" \
    "${CARGO_HOME}/.global-cache" \
    "${CARGO_HOME}/.package-cache" \
    "${CARGO_HOME}/.package-cache-mutate"
