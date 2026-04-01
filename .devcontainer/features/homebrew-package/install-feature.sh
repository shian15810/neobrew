#!/bin/sh

set -euvx

LC_ALL="${LC_ALL:?}"

HOMEBREW_NO_ANALYTICS="${HOMEBREW_NO_ANALYTICS:?}"
HOMEBREW_NO_ENV_HINTS="${HOMEBREW_NO_ENV_HINTS:?}"

brew analytics off

brew update

brew untap homebrew/core || true
brew untap homebrew/cask || true

brew upgrade

brew install "$@"

brew autoremove
brew cleanup --prune=all --scrub

rm -rf -- "$(brew --cache)"
