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
#
# Syncs (or checks the sync status of) the "score-*" skill directories that
# score_tooling ships under .github/skills into a downstream repository's own
# .github/skills directory.
#
# Usage:
#   sync_skills.sh sync  <tooling-skill-files...>
#   sync_skills.sh check <tooling-skill-files...> -- <repo-skill-files...>
#
# "tooling-skill-files" are the runfiles paths of the files contained in
# @score_tooling//.github/skills:skills (i.e. the upstream, canonical copies).
# "repo-skill-files" (check mode only) are the paths, relative to the
# downstream repository root, of the files currently committed under
# .github/skills/score-*/** in that repository.

set -euo pipefail

SKILL_MARKER=".github/skills"

die() {
    echo "error: $*" >&2
    exit 2
}

# Given an absolute/runfiles path to a file inside a "*/.github/skills/..."
# tree, prints the path relative to (and including) the skill directory name,
# e.g. ".../external/score_tooling+/.github/skills/score-testing/SKILL.md"
# becomes "score-testing/SKILL.md".
relative_skill_path() {
    local f="$1"
    case "$f" in
        *"${SKILL_MARKER}/"*)
            echo "${f#*${SKILL_MARKER}/}"
            ;;
        *)
            die "path '$f' does not contain '${SKILL_MARKER}/'"
            ;;
    esac
}

cmd_sync() {
    local dest_root="${BUILD_WORKSPACE_DIRECTORY:-}"
    [ -n "$dest_root" ] || die "must be run via 'bazel run', BUILD_WORKSPACE_DIRECTORY is not set"
    dest_root="${dest_root}/.github/skills"
    mkdir -p "$dest_root"

    declare -A upstream_dirs=()

    local f rel dir_name dest
    for f in "$@"; do
        rel="$(relative_skill_path "$f")"
        dir_name="${rel%%/*}"
        upstream_dirs["$dir_name"]=1

        dest="${dest_root}/${rel}"
        mkdir -p "$(dirname "$dest")"
        cp -f "$f" "$dest"
    done

    # Remove skill directories that score_tooling no longer ships, so stale
    # skills do not linger after an upstream removal/rename.
    local d name
    for d in "$dest_root"/score-*; do
        [ -d "$d" ] || continue
        name="$(basename "$d")"
        if [ -z "${upstream_dirs[$name]:-}" ]; then
            echo "Removing stale score_tooling skill: ${name}"
            rm -rf "$d"
        fi
    done

    echo "Synced score_tooling skills: ${!upstream_dirs[*]}"
}

cmd_check() {
    local tooling_files=()
    local repo_files=()
    local seen_separator=0

    local a
    for a in "$@"; do
        if [ "$a" = "--" ]; then
            seen_separator=1
            continue
        fi
        if [ "$seen_separator" -eq 0 ]; then
            tooling_files+=("$a")
        else
            repo_files+=("$a")
        fi
    done

    declare -A upstream_map=()
    local f rel
    for f in "${tooling_files[@]}"; do
        rel="$(relative_skill_path "$f")"
        upstream_map["$rel"]="$f"
    done

    declare -A repo_map=()
    for f in "${repo_files[@]}"; do
        case "$f" in
            "${SKILL_MARKER}/"score-*)
                rel="${f#${SKILL_MARKER}/}"
                repo_map["$rel"]="$f"
                ;;
        esac
    done

    local status=0
    local up cf
    for rel in "${!upstream_map[@]}"; do
        up="${upstream_map[$rel]}"
        cf="${repo_map[$rel]:-}"
        if [ -z "$cf" ]; then
            echo "MISSING:   ${SKILL_MARKER}/${rel}"
            status=1
        elif ! diff -q "$up" "$cf" >/dev/null 2>&1; then
            echo "OUT OF DATE: ${SKILL_MARKER}/${rel}"
            status=1
        fi
        unset "repo_map[$rel]"
    done

    for rel in "${!repo_map[@]}"; do
        echo "STALE (no longer provided by score_tooling): ${SKILL_MARKER}/${rel}"
        status=1
    done

    if [ "$status" -ne 0 ]; then
        echo ""
        echo "score_tooling skills are out of sync. Run: bazel run //:sync_skills"
        exit 1
    fi

    echo "score_tooling skills are up to date."
}

mode="${1:-}"
[ -n "$mode" ] || die "usage: sync_skills.sh <sync|check> ..."
shift

# Note: $(locations label) in the "args" attribute expands to one argv entry
# per file (not a single space-joined string), so remaining args can be used
# as-is (no manual blob-splitting needed).
case "$mode" in
    sync)
        cmd_sync "$@"
        ;;
    check)
        # Remaining args are the tooling files, a "--" separator, then the
        # repo's own committed files (from a plain glob(), not location-expanded).
        cmd_check "$@"
        ;;
    *)
        die "unknown mode '$mode' (expected 'sync' or 'check')"
        ;;
esac
