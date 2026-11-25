#!/usr/bin/env bash
# *******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# *******************************************************************************

set -euo pipefail

if [[ -z "${BIT_BAZEL_BINARY:-}" || -z "${BIT_WORKSPACE_DIR:-}" || -z "${TEST_TMPDIR:-}" ]]; then
    echo "ERROR: BIT_BAZEL_BINARY, BIT_WORKSPACE_DIR, and TEST_TMPDIR must be set" >&2
    exit 1
fi

bazel_binary="$BIT_BAZEL_BINARY"
workspace_dir="$BIT_WORKSPACE_DIR"
test_tmpdir="$TEST_TMPDIR"

# The nested Bazel must not see the outer test variables. RUNFILES_* stay: the
# Bazel launcher script needs them.
for name in $(compgen -e); do
    case "$name" in
        TEST_* | BIT_* | XML_OUTPUT_FILE) unset "$name" ;;
    esac
done

module_file="$workspace_dir/MODULE.bazel"
if [[ ! -f "$module_file" ]]; then
    echo "ERROR: module file not found: $module_file" >&2
    exit 1
fi

source_dir="$(dirname "$(readlink -f "$module_file")")"
workspace_copy="$test_tmpdir/ws"
output_root="$test_tmpdir/root"
log_file="$test_tmpdir/l4-child.log"
child_status=0

if [[ -n "${L4_CACHE_DIR:-}" ]]; then
    bazelisk_home="$L4_CACHE_DIR/bazelisk"
else
    bazelisk_home="$test_tmpdir/bazelisk"
fi
mkdir -p "$bazelisk_home"
export BAZELISK_HOME="$bazelisk_home"

bz() {
    "$bazel_binary" \
        --output_user_root="$output_root" \
        --output_base="$output_root/output-base" \
        "$@"
}

# Runs the nested Bazel, streaming its output to stdout and $log_file; exit code in $child_status.
run_logged() {
    set +e
    bz "$@" 2>&1 | tee "$log_file"
    child_status="${PIPESTATUS[0]}"
    set -e
}

cleanup() {
    status=$?
    trap - EXIT
    set +e
    cleanup_failed=0

    if [[ -d "$output_root" ]]; then
        if ! bz shutdown; then
            echo "ERROR: nested Bazel shutdown failed" >&2
            cleanup_failed=1
        fi
    fi

    for path in "$output_root" "$workspace_copy"; do
        if [[ -e "$path" ]]; then
            if ! chmod -R u+w "$path"; then
                echo "ERROR: could not make temporary path writable: $path" >&2
                cleanup_failed=1
            fi
            if ! rm -rf -- "$path"; then
                echo "ERROR: could not remove temporary path: $path" >&2
                cleanup_failed=1
            fi
        fi
    done

    if [[ "$status" -eq 0 && "$cleanup_failed" -ne 0 ]]; then
        status=1
    fi
    exit "$status"
}
trap cleanup EXIT

mkdir -p "$workspace_copy"
tar \
    --exclude='./bazel-*' \
    --exclude='./user.bazelrc' \
    --exclude='./MODULE.bazel.lock' \
    --exclude='./.bazelversion' \
    -C "$source_dir" \
    -cf - . | tar -C "$workspace_copy" -xf -

override_args=()
declare -A override_paths=()
read -r -a override_entries <<<"${OVERRIDE_MODULES:-}"
for entry in "${override_entries[@]}"; do
    if [[ "$entry" != *=* ]]; then
        echo "ERROR: invalid module override: $entry" >&2
        exit 1
    fi
    module_name="${entry%%=*}"
    module_path="${entry#*=}"
    if [[ -z "$module_name" || -z "$module_path" || ! -f "$module_path" ]]; then
        echo "ERROR: invalid module override file: $entry" >&2
        exit 1
    fi
    override_path="$(dirname "$(readlink -f "$module_path")")"
    override_args+=("--override_module=$module_name=$override_path")
    override_paths["$module_name"]="$override_path"
done

# The consumer's relative local_path_override paths do not exist in the copy. They
# must be overridden, and must point to the same module as the injected override.
declared_overrides="$(awk '
    /^local_path_override\(/ { in_block = 1; name = ""; path = "" }
    in_block && match($0, /module_name *= *"[^"]+"/) {
        name = substr($0, RSTART, RLENGTH)
        sub(/^[^"]*"/, "", name)
        sub(/"$/, "", name)
    }
    in_block && match($0, /path *= *"[^"]+"/) {
        path = substr($0, RSTART, RLENGTH)
        sub(/^[^"]*"/, "", path)
        sub(/"$/, "", path)
    }
    in_block && /\)[[:space:]]*$/ { print name, path; in_block = 0 }
' "$workspace_copy/MODULE.bazel")"
while read -r module_name declared_path; do
    [[ -n "$module_name" ]] || continue
    if [[ -z "${override_paths[$module_name]:-}" ]]; then
        echo "ERROR: MODULE.bazel uses local_path_override for '$module_name'; add it to l4_test(overrides = [...])" >&2
        exit 1
    fi
    [[ "$declared_path" == /* ]] || declared_path="$source_dir/$declared_path"
    resolved_path="$(readlink -f -- "$declared_path")"
    if [[ "$resolved_path" != "${override_paths[$module_name]}" ]]; then
        echo "ERROR: local_path_override for '$module_name' resolves to $resolved_path, but l4_test overrides it with ${override_paths[$module_name]}" >&2
        exit 1
    fi
done <<<"$declared_overrides"

cases_file=""
if [[ -n "${L4_CASES:-}" ]]; then
    if [[ ! -f "$L4_CASES" ]]; then
        echo "ERROR: case file not found: $L4_CASES" >&2
        exit 1
    fi
    cases_file="$(readlink -f "$L4_CASES")"
fi

common_args=(
    --lockfile_mode=update
    --color=no
    --curses=no
    "${override_args[@]}"
)
if [[ -n "${L4_CACHE_DIR:-}" ]]; then
    mkdir -p "$L4_CACHE_DIR/repo" "$L4_CACHE_DIR/disk"
    common_args+=(
        "--repository_cache=$L4_CACHE_DIR/repo"
        "--disk_cache=$L4_CACHE_DIR/disk"
    )
fi
# user.bazelrc is not copied, so the CI download mirror is passed explicitly.
if [[ -n "${L4_DOWNLOADER_CONFIG:-}" ]]; then
    if [[ ! -f "$L4_DOWNLOADER_CONFIG" ]]; then
        echo "ERROR: downloader config not found: $L4_DOWNLOADER_CONFIG" >&2
        exit 1
    fi
    common_args+=("--downloader_config=$L4_DOWNLOADER_CONFIG")
fi

run_stage() {
    local started=$SECONDS
    stage="$1"
    shift
    printf '===== L4 STAGE %s: bazel' "$stage"
    printf ' %q' "$@"
    printf ' =====\n'
    run_logged "$@"
    if [[ "$child_status" -ne 0 ]]; then
        printf '===== L4 STAGE %s: FAILED (exit %s, %ss) =====\n' \
            "$stage" "$child_status" "$((SECONDS - started))"
        return "$child_status"
    fi
    printf '===== L4 STAGE %s: OK (%ss) =====\n' "$stage" "$((SECONDS - started))"
}

run_example_mode() {
    run_stage build build //... "${common_args[@]}" || return $?
    run_stage test test //... --test_output=errors "${common_args[@]}" || return $?

    if [[ -n "${L4_VARIANT_BUILD:-}" ]]; then
        local -a variant_build=()
        read -r -a variant_build <<<"$L4_VARIANT_BUILD"
        run_stage build_variant build "${variant_build[@]}" "${common_args[@]}" || return $?
    fi
}

tab=$'\t'
case_row_re="^[^${tab}]+(${tab}[^${tab}]+){4}\$"
case_names=()
case_commands=()
case_targets=()
case_expectations=()
case_patterns=()
declare -A seen_cases=()

# Validates every row before any nested Bazel command runs.
load_cases() {
    local line line_no=0 previous="" rc
    local name command target expected pattern

    while IFS= read -r line || [[ -n "$line" ]]; do
        line_no=$((line_no + 1))
        line="${line%$'\r'}"
        if [[ -z "$line" || "$line" == \#* ]]; then
            continue
        fi

        if ! [[ "$line" =~ $case_row_re ]]; then
            echo "ERROR: $cases_file:$line_no: expected 5 non-empty tab-separated fields" >&2
            return 1
        fi
        IFS=$'\t' read -r name command target expected pattern <<<"$line"

        if [[ "$command" != "build" && "$command" != "test" && "$command" != "run" ]]; then
            echo "ERROR: $cases_file:$line_no: unsupported command '$command'" >&2
            return 1
        fi
        if [[ "$expected" != "pass" && "$expected" != "fail" ]]; then
            echo "ERROR: $cases_file:$line_no: unsupported expectation '$expected'" >&2
            return 1
        fi
        if [[ "$expected" == "fail" && "$pattern" == "-" ]]; then
            echo "ERROR: $cases_file:$line_no: expected failure needs an output pattern" >&2
            return 1
        fi
        if [[ "$pattern" != "-" ]]; then
            rc=0
            grep -Eq -- "$pattern" </dev/null || rc=$?
            if [[ "$rc" -eq 2 ]]; then
                echo "ERROR: $cases_file:$line_no: invalid pattern '$pattern'" >&2
                return 1
            fi
        fi

        if [[ "$name" != "$previous" ]]; then
            if [[ -n "${seen_cases[$name]:-}" ]]; then
                echo "ERROR: $cases_file:$line_no: rows for case '$name' must be contiguous" >&2
                return 1
            fi
            seen_cases[$name]=1
            previous="$name"
        fi

        case_names+=("$name")
        case_commands+=("$command")
        case_targets+=("$target")
        case_expectations+=("$expected")
        case_patterns+=("$pattern")
    done <"$cases_file"

    if [[ "${#case_names[@]}" -eq 0 ]]; then
        echo "ERROR: no cases in $cases_file" >&2
        return 1
    fi
}

run_cases_mode() {
    load_cases || return $?

    local current_case="" case_failed=0 failed_cases=() summary=""
    local i case_name command target expected pattern command_args actual verdict line_failed

    for i in "${!case_names[@]}"; do
        case_name="${case_names[$i]}"
        command="${case_commands[$i]}"
        target="${case_targets[$i]}"
        expected="${case_expectations[$i]}"
        pattern="${case_patterns[$i]}"

        if [[ "$case_name" != "$current_case" ]]; then
            current_case="$case_name"
            case_failed=0
        fi
        if [[ "$case_failed" -ne 0 ]]; then
            printf '===== L4 CASE %s %s %s SKIPPED (earlier line failed) =====\n' \
                "$case_name" "$command" "$target"
            continue
        fi

        printf '===== L4 CASE %s %s %s expect=%s =====\n' \
            "$case_name" "$command" "$target" "$expected"
        command_args=("$command" "$target")
        if [[ "$command" == "test" ]]; then
            command_args+=(--test_output=errors)
        fi
        command_args+=("${common_args[@]}")

        run_logged "${command_args[@]}"
        actual="pass"
        if [[ "$child_status" -ne 0 ]]; then
            actual="fail"
        fi

        line_failed=1
        if [[ "$actual" != "$expected" ]]; then
            if [[ "$actual" == "pass" ]]; then
                verdict="UNEXPECTED_PASS"
            else
                verdict="UNEXPECTED_FAIL"
            fi
        elif [[ "$pattern" != "-" ]] && ! grep -Eq -- "$pattern" "$log_file"; then
            verdict="PATTERN_MISSING"
        else
            verdict="PASS"
            line_failed=0
        fi

        printf '===== L4 CASE %s %s: %s (exit %s) =====\n' \
            "$case_name" "$command" "$verdict" "$child_status"
        if [[ "$line_failed" -ne 0 ]]; then
            case_failed=1
            failed_cases+=("$case_name($command)")
        fi
    done

    if [[ "${#failed_cases[@]}" -eq 0 ]]; then
        printf 'L4 SUMMARY: %s cases, 0 failed\n' "${#seen_cases[@]}"
        return 0
    fi

    for case_name in "${failed_cases[@]}"; do
        summary+="${summary:+, }$case_name"
    done
    printf 'L4 SUMMARY: %s cases, %s failed: %s\n' \
        "${#seen_cases[@]}" "${#failed_cases[@]}" "$summary"
    return 1
}

cd "$workspace_copy"
printf 'L4 Bazelisk home: %s\n' "$bazelisk_home"
printf 'L4 output base: %s\n' "$output_root/output-base"
if [[ -n "$cases_file" ]]; then
    run_cases_mode
else
    run_example_mode
fi
