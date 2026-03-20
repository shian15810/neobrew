#!/bin/sh

set -euvx

export HOMEBREW_NO_ANALYTICS=1
export HOMEBREW_NO_ENV_HINTS=1

brew analytics off

brew update

brew untap homebrew/core || true
brew untap homebrew/cask || true

brew upgrade

brew install "$@"

brew autoremove
brew cleanup --prune=all --scrub

rm -rf -- "$(brew --cache)"
