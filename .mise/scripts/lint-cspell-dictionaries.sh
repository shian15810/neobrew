#!/bin/sh

set -eu

export LC_ALL=C

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd -P)"

cd -- "${SCRIPT_DIR}/../.."

ACTUAL="$(
    find -L .cspell/. ! -name . -prune -name '*.txt' -type f -exec cat {} + \
        || true
)"
ACTUAL="$(
    printf '%s' "$ACTUAL" \
        | grep -v -e'^[^[:graph:]]*$' -e'^[^[:graph:]]*#' || true
)"
ACTUAL="$(
    printf '%s' "$ACTUAL" \
        | sed -e's/#.*//' -e's/^[^[:graph:]]*//' -e's/[^[:graph:]]*$//'
)"
ACTUAL="$(printf '%s' "$ACTUAL" | LC_ALL='' sort -f)"

EXPECTED="$(cspell dictionaries --no-show-location --enabled)"

EXPECTED_HEADER="$(printf '%s' "$EXPECTED" | head -n1)"

if [ "$EXPECTED_HEADER" != "Dictionary" ]; then
    exit 70
fi

EXPECTED="$(printf '%s' "$EXPECTED" | tail -n+2)"
EXPECTED="$(
    printf '%s' "$EXPECTED" | grep '^custom-[[:graph:]]\{1,\}\*$' || true
)"
EXPECTED="$(
    printf '%s' "$EXPECTED" | sed -e's/\*$//' -e's/^/--disable-dictionary=/'
)"

if [ -n "$EXPECTED" ]; then
    EXPECTED="$(printf '%s' "$EXPECTED" | sed 's/[^[:alnum:]]/\\&/g')"
    EXPECTED="$(
        printf '%s' "$EXPECTED" \
            | xargs -E '' cspell . \
                --no-must-find-files \
                --no-progress \
                --no-summary \
                --no-exit-code \
                --words-only \
                --unique
    )"
else
    EXPECTED="$(
        cspell . \
            --no-must-find-files \
            --no-progress \
            --no-summary \
            --no-exit-code \
            --words-only \
            --unique
    )"
fi

EXPECTED="$(printf '%s' "$EXPECTED" | LC_ALL='' sort -f)"

printf '%s\n' "$ACTUAL" | diff -u - /dev/fd/3 3<<- EOF
	$EXPECTED
EOF
