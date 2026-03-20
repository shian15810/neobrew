#!/bin/sh

set -eu

if [ "$(uname -s)" = "Darwin" ]; then
    export -- "$(rustc --print=deployment-target)"
fi
