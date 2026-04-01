#!/bin/sh

set -euvx

export LC_ALL=C

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd -P)"

SCCACHE_DIR="${SCCACHE_DIR:?}"

HOMEBREW_DEVCONTAINER="${HOMEBREW_DEVCONTAINER:?}"

cd -- "${SCRIPT_DIR}/../.."

sudo chown -- "$(id -un):$(id -gn)" "$SCCACHE_DIR"

brew bundle

mise trust
mise install
mise upgrade

rustup self update
rustup update

rustup --version
cargo --version
rustc --version

rustup +nightly --version
cargo +nightly --version
rustc +nightly --version
