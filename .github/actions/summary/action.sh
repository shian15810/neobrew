#!/bin/sh

set -eu

export LC_ALL="C"

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd -P)"

INPUT_STEP_ID="${INPUT_STEP_ID:-""}"
INPUT_INPUTS="${INPUT_INPUTS:-""}"
INPUT_STEPS="${INPUT_STEPS:-""}"

GITHUB_STEP_SUMMARY="${GITHUB_STEP_SUMMARY:?}"

cd -- "${SCRIPT_DIR}/../../.."

INPUT_OUTPUTS="$(
    printf '%s' "$INPUT_STEPS" \
        | jq --compact-output --arg step_id "$INPUT_STEP_ID" '.
            | select(. == "") // (.
                | (select($step_id != "") | .[$step_id].outputs) // (.
                    | with_entries(.key as $step_id
                        | .value = (.value.outputs
                            | with_entries(.key as $output_id
                                | .key = $step_id + "." + $output_id
                            )
                        )
                    ) | add
                )
            )
        '
)"

GITHUB_STEP_SUMMARY_INPUTS_TABLE_DATA="$(
    printf '%s' "$INPUT_INPUTS" \
        | jq --raw-output '.
            | select(. == "") // (.
                | to_entries[]
                | map_values((select(length > 0) | " `" + . + "` ") // " ")
                | "|" + .key + "|" + .value + "|"
            )
        '
)"

GITHUB_STEP_SUMMARY_OUTPUTS_TABLE_DATA="$(
    printf '%s' "$INPUT_OUTPUTS" \
        | jq --raw-output '.
            | select(. == "") // (.
                | to_entries[]
                | map_values((select(length > 0) | " `" + . + "` ") // " ")
                | "|" + .key + "|" + .value + "|"
            )
        '
)"

GITHUB_STEP_SUMMARY_INPUTS_DETAILS_JSON="$(
    printf '%s' "$INPUT_INPUTS" \
        | jq '.
            | select(. == "") // (.
                | with_entries(.)
            )
        '
)"

GITHUB_STEP_SUMMARY_OUTPUTS_DETAILS_JSON="$(
    printf '%s' "$INPUT_OUTPUTS" \
        | jq '.
            | select(. == "") // (.
                | with_entries(.)
            )
        '
)"

GITHUB_STEP_SUMMARY_HEADING="$(
    if [ -n "$INPUT_STEP_ID" ]; then
        echo "## \`${INPUT_STEP_ID}\`"
    fi
)"

GITHUB_STEP_SUMMARY_INPUTS_TABLE="$(
    if [ -n "$GITHUB_STEP_SUMMARY_INPUTS_TABLE_DATA" ]; then
        echo "| Input | Value |"
        echo "| --- | --- |"
        echo "$GITHUB_STEP_SUMMARY_INPUTS_TABLE_DATA"
    fi
)"

GITHUB_STEP_SUMMARY_OUTPUTS_TABLE="$(
    if [ -n "$GITHUB_STEP_SUMMARY_OUTPUTS_TABLE_DATA" ]; then
        echo "| Output | Value |"
        echo "| --- | --- |"
        echo "$GITHUB_STEP_SUMMARY_OUTPUTS_TABLE_DATA"
    fi
)"

GITHUB_STEP_SUMMARY_INPUTS_DETAILS="$(
    if [ -n "$GITHUB_STEP_SUMMARY_INPUTS_DETAILS_JSON" ]; then
        echo "<details>"
        echo ""
        echo "<summary>Full JSON Inputs</summary>"
        echo ""
        echo '```json'
        echo "$GITHUB_STEP_SUMMARY_INPUTS_DETAILS_JSON"
        echo '```'
        echo ""
        echo "</details>"
    fi
)"

GITHUB_STEP_SUMMARY_OUTPUTS_DETAILS="$(
    if [ -n "$GITHUB_STEP_SUMMARY_OUTPUTS_DETAILS_JSON" ]; then
        echo "<details>"
        echo ""
        echo "<summary>Full JSON Outputs</summary>"
        echo ""
        echo '```json'
        echo "$GITHUB_STEP_SUMMARY_OUTPUTS_DETAILS_JSON"
        echo '```'
        echo ""
        echo "</details>"
    fi
)"

{
    GITHUB_STEP_SUMMARY_HAS_PREVIOUS_CONTENT=false

    if [ -n "$GITHUB_STEP_SUMMARY_HEADING" ]; then
        echo "$GITHUB_STEP_SUMMARY_HEADING"

        GITHUB_STEP_SUMMARY_HAS_PREVIOUS_CONTENT=true
    fi

    if [ -n "$GITHUB_STEP_SUMMARY_INPUTS_TABLE" ]; then
        if [ "$GITHUB_STEP_SUMMARY_HAS_PREVIOUS_CONTENT" = true ]; then
            echo ""
        fi

        echo "$GITHUB_STEP_SUMMARY_INPUTS_TABLE"

        GITHUB_STEP_SUMMARY_HAS_PREVIOUS_CONTENT=true
    fi

    if [ -n "$GITHUB_STEP_SUMMARY_OUTPUTS_TABLE" ]; then
        if [ "$GITHUB_STEP_SUMMARY_HAS_PREVIOUS_CONTENT" = true ]; then
            echo ""
        fi

        echo "$GITHUB_STEP_SUMMARY_OUTPUTS_TABLE"

        GITHUB_STEP_SUMMARY_HAS_PREVIOUS_CONTENT=true
    fi

    if [ -n "$GITHUB_STEP_SUMMARY_INPUTS_DETAILS" ]; then
        if [ "$GITHUB_STEP_SUMMARY_HAS_PREVIOUS_CONTENT" = true ]; then
            echo ""
        fi

        echo "$GITHUB_STEP_SUMMARY_INPUTS_DETAILS"

        GITHUB_STEP_SUMMARY_HAS_PREVIOUS_CONTENT=true
    fi

    if [ -n "$GITHUB_STEP_SUMMARY_OUTPUTS_DETAILS" ]; then
        if [ "$GITHUB_STEP_SUMMARY_HAS_PREVIOUS_CONTENT" = true ]; then
            echo ""
        fi

        echo "$GITHUB_STEP_SUMMARY_OUTPUTS_DETAILS"
    fi
} >> "$GITHUB_STEP_SUMMARY"
