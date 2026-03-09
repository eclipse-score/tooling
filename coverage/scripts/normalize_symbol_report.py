#!/usr/bin/env python3
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
import json
import sys
from pathlib import Path

if len(sys.argv) < 2:
    print(
        "usage: normalize_symbol_report.py <symbol_report_json> [roots...]",
        file=sys.stderr,
    )
    sys.exit(2)

json_path = Path(sys.argv[1])
roots = []
for arg in sys.argv[2:]:
    if arg:
        roots.append(Path(arg).resolve())

with json_path.open("r", encoding="utf-8") as fh:
    data = json.load(fh)


def relativize(p: Path):
    if not p.is_absolute():
        return p.as_posix()
    for root in roots:
        try:
            return p.relative_to(root).as_posix()
        except ValueError:
            pass
    try:
        rp = p.resolve()
    except Exception:
        return None
    for root in roots:
        try:
            return rp.relative_to(root).as_posix()
        except ValueError:
            pass
    return None


changed = False
symbols = []
for sym in data.get("symbols", []):
    fname = sym.get("filename")
    if not fname:
        continue
    rel = relativize(Path(fname))
    if rel is None:
        changed = True
        continue
    if rel != fname:
        sym["filename"] = rel
        changed = True
    symbols.append(sym)

if symbols != data.get("symbols", []):
    data["symbols"] = symbols

if changed:
    with json_path.open("w", encoding="utf-8") as fh:
        json.dump(data, fh)
