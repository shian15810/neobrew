#!/bin/sh

set -euvx

export LC_ALL="C"

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd -P)"

cd -- "${SCRIPT_DIR}/../.."

HOMEBREW_BUNDLE_BREW_SKIP="${HOMEBREW_BUNDLE_BREW_SKIP:?}"
MISE_USE_VERSIONS_HOST_TRACK="${MISE_USE_VERSIONS_HOST_TRACK:?}"
MISE_DISABLE_HINTS="${MISE_DISABLE_HINTS:?}"
MISE_EXPERIMENTAL="${MISE_EXPERIMENTAL:?}"
SCCACHE_DIR="${SCCACHE_DIR:?}"

export MISE_ENV="development"
export MISE_YES=true

sudo chown -- "$(id -un):$(id -gn)" "$SCCACHE_DIR"

brew bundle

rustup self update
rustup update

cargo +nightly miri setup

mise --env="development" --yes trust
mise --env="development" --yes install
mise --env="development" --yes upgrade

rustup +stable show --verbose
rustup +stable --version --verbose
rustc +stable --version --verbose
cargo +stable --version --verbose
rustup +stable component list --installed

rustup +nightly show --verbose
rustup +nightly --version --verbose
rustc +nightly --version --verbose
cargo +nightly --version --verbose
rustup +nightly component list --installed
