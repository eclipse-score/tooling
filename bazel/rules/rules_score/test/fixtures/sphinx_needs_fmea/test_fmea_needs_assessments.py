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
"""Tests for fmea_needs_assessments."""

import json
import os
import tempfile
import unittest

from fmea_needs_assessments import main
from lobster.common.errors import Message_Handler
from lobster.common.io import lobster_read, lobster_write
from lobster.common.items import Requirement, Tracing_Tag
from lobster.common.location import File_Reference


def _item(name, refs=(), version=None):
    item = Requirement(
        tag=Tracing_Tag("req", name, version),
        location=File_Reference("x.trlc", line=1),
        framework="TRLC",
        kind="Test",
        name=name,
        text="",
        status=None,
    )
    for ref in refs:
        item.add_tracing_target(Tracing_Tag("req", ref))
    return item


def _fmea(**overrides):
    need = {
        "type": "comp_saf_fmea",
        "title": "No return",
        "content": "The watchdog detects it.",
        "mitigated_by": ["comp_req__lib__watchdog[version==2]", "aou_req__platform__error_reaction"],
        "sufficient": "yes",
        "status": "valid",
        "version": 3,
        "docname": "safety_analysis/fmea",
        "doctype": ".rst",
        "lineno": 7,
    }
    need.update(overrides)
    return need


class FmeaNeedsAssessmentsTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = tmp.name
        self.rc = self._lobster(
            "rc.lobster",
            [
                _item("Fta.Crash", ["Fmea.comp_saf_fmea__lib__no_return"]),
                _item("Fta.Hang", ["Fmea.comp_saf_fmea__lib__no_return"]),
                _item("Fta.Other", ["Fmea.comp_saf_fmea__lib__other"]),
            ],
        )
        self.measures = self._lobster("measures.lobster", [_item("comp_req__lib__watchdog", version=2)])

    def _lobster(self, name, items):
        path = os.path.join(self.dir, name)
        with open(path, "w", encoding="utf-8") as f:
            lobster_write(f, Requirement, "test", items)
        return path

    def _run(self, **needs):
        needs_path = os.path.join(self.dir, "needs.json")
        with open(needs_path, "w", encoding="utf-8") as f:
            json.dump({"current_version": "1", "versions": {"1": {"needs": needs}}}, f)
        out = os.path.join(self.dir, "assessments.lobster")
        argv = [needs_path, "--package", "Fmea", "--root-causes", self.rc, "--measures", self.measures, "--out", out]
        self.assertEqual(0, main(argv))
        items = {}
        lobster_read(Message_Handler(), out, "assessments", items)
        return items

    def test_sufficient_need_assesses_its_root_causes_and_traces_known_measures(self):
        items = self._run(comp_saf_fmea__lib__no_return=_fmea())
        self.assertEqual(["req Fmea.comp_saf_fmea__lib__no_return.assessment"], list(items))
        item = items["req Fmea.comp_saf_fmea__lib__no_return.assessment"]
        self.assertEqual("3", str(item.tag.version))
        self.assertEqual("The watchdog detects it.", item.text)
        refs = sorted((t.key(), t.version) for t in item.unresolved_references)
        self.assertEqual(
            [("req Fta.Crash", None), ("req Fta.Hang", None), ("req comp_req__lib__watchdog", "2")],
            [(k, None if v is None else str(v)) for k, v in refs],
        )

    def test_insufficient_need_leaves_its_root_causes_unassessed(self):
        self.assertEqual({}, self._run(comp_saf_fmea__lib__no_return=_fmea(sufficient="no")))

    def test_need_without_root_causes_has_no_assessment(self):
        self.assertEqual({}, self._run(comp_saf_fmea__lib__unknown=_fmea()))

    def test_invalid_and_other_needs_are_ignored(self):
        items = self._run(
            comp_saf_fmea__lib__no_return=_fmea(status="invalid"),
            comp_saf_fmea__lib__other=_fmea(type="comp_saf_dfa"),
        )
        self.assertEqual({}, items)


if __name__ == "__main__":
    unittest.main()
