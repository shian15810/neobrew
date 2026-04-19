#!/bin/sh

set -euvx

export LC_ALL="C"

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd -P)"

export MISE_ENV="development"
export MISE_YES=true

HOMEBREW_BUNDLE_BREW_SKIP="${HOMEBREW_BUNDLE_BREW_SKIP:?}"
SCCACHE_DIR="${SCCACHE_DIR:?}"

cd -- "${SCRIPT_DIR}/../.."

sudo chown -- "$(id -un):$(id -gn)" "$SCCACHE_DIR"

brew bundle

mise --env="development" --yes trust
mise --env="development" --yes install
mise --env="development" --yes upgrade

rustup self update
rustup update

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
