#!/bin/sh

set -euvx

LC_ALL="${LC_ALL:?}"

RUSTUP_PERMIT_COPY_RENAME="${RUSTUP_PERMIT_COPY_RENAME:?}"

RUSTUP_HOME="${RUSTUP_HOME:?}"
CARGO_HOME="${CARGO_HOME:?}"

rustup self update

DEFAULT_TOOLCHAIN="$(rustup default)"
DEFAULT_TOOLCHAIN="$(printf '%s' "$DEFAULT_TOOLCHAIN" | awk '{ print $1 }')"

rustup toolchain link "system" -- \
    "${RUSTUP_HOME}/toolchains/${DEFAULT_TOOLCHAIN}"
rustup default "system"

rustup update

rustup toolchain install "$@"

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
