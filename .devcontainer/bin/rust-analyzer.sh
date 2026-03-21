#!/bin/sh

set -eu

exec "$(brew --prefix)/bin/rust-analyzer" "$@"
