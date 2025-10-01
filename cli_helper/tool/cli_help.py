# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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
import os, subprocess, shutil, textwrap, xml.etree.ElementTree as ET


def bazel(*args: str) -> str:
    try:
        output = subprocess.check_output(["bazel", *args], text=True)
    except subprocess.CalledProcessError as e:
        print(f"\033[31m !! Error in bazel query, help string is incomplete!!:\033[0m")
        output = e.output
    return output


# When invoked via `bazel run`, Bazel sets this env var
ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
if ws:
    os.chdir(ws)

# build a single query for all cli_help tags
expr = 'kind("rule", attr(tags, "cli_help=.*", deps(//...)))'

xml = bazel(
    "query",
    expr,
    "--output=xml",
    "--keep_going",
    "--ui_event_filters=-INFO,-progress",
    "--noshow_progress",
)

root = ET.fromstring(xml)

# format the output
rows, max_len = [], 0
for rule in root.iter("rule"):
    label = rule.get("name")
    # define here if you want to skip some targets
    if label.endswith(".find_main"):
        continue

    tags = [n.get("value") for n in rule.findall("list[@name='tags']/*")]
    for t in tags:
        if t.startswith("cli_help="):
            desc = t.split("=", 1)[1]
            rows.append((label, desc))
            max_len = max(max_len, len(label))
            break

# pretty-print
col_w = (max_len + 2) if rows else 2
term_cols = shutil.get_terminal_size(fallback=(80, 24)).columns
descr_w = max(20, term_cols - col_w - 1)

CYAN = "\033[36m"
print(f"{CYAN}BAZEL TARGETS:\n")

for label, desc in sorted(rows):
    paragraphs = desc.splitlines()
    first, *rest = paragraphs or [""]
    wrapped_first = textwrap.wrap(
        first, width=descr_w, break_long_words=False, break_on_hyphens=False
    ) or [""]
    print(f"{label.ljust(col_w)} {wrapped_first[0]}")
    for line in wrapped_first[1:]:
        print(" " * col_w + " " + line)
    for para in rest:
        print(" " * col_w + " " + para)
