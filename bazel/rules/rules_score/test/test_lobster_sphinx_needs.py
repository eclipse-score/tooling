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
"""Tests for lobster_sphinx_needs."""

import json
import os
import tempfile
import unittest

from lobster.common.errors import Message_Handler
from lobster.common.io import lobster_read

from lobster_sphinx_needs import ConfigError, convert, load_config, main, parse_link

CONFIG = """\
conversion-rules:
  - need-type: comp_req
    kind: CompReq
    version-field: version
    status-field: status
    exclude-status: [invalid]
    tags:
      - derived_from
      - covers
"""

NEEDS = {
    "comp_req__bits__set": {
        "id": "comp_req__bits__set",
        "type": "comp_req",
        "title": "Set bits",
        "content": "The component shall set *bits*.",
        "version": 2,
        "status": "valid",
        "safety": "ASIL_B",
        "docname": "requirements/index",
        "lineno": 33,
        "derived_from": ["feat_req__bits[version==3]"],
        "covers": ["aou_req__bits__ints"],
    },
    "comp_req__bits__old": {
        "id": "comp_req__bits__old",
        "type": "comp_req",
        "title": "Old",
        "content": "",
        "version": 1,
        "status": "invalid",
    },
    "aou_req__bits__ints": {"id": "aou_req__bits__ints", "type": "aou_req", "title": "Ints", "version": 1},
}


def _write(directory, name, content):
    path = os.path.join(directory, name)
    with open(path, "w", encoding="utf-8") as f:
        f.write(content if isinstance(content, str) else json.dumps(content))
    return path


def _needs_json(needs, version="1.0"):
    return {"current_version": version, "versions": {version: {"needs": needs}}}


class ParseLinkTest(unittest.TestCase):
    def test_plain_id(self):
        self.assertEqual(str(parse_link("feat_req__x", "req")), "req feat_req__x")

    def test_version_filter_becomes_tag_version(self):
        self.assertEqual(str(parse_link("feat_req__x[version==2]", "req")), "req feat_req__x@2")

    def test_other_filters_are_ignored(self):
        self.assertEqual(str(parse_link("feat_req__x[status=='valid']", "req")), "req feat_req__x")


class ConvertTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.config = load_config(_write(self.tmp.name, "config.yaml", CONFIG))

    def test_converts_matching_needs_like_lobster_trlc(self):
        needs = _write(self.tmp.name, "needs.json", _needs_json(NEEDS))
        converted = convert([needs], self.config)

        self.assertEqual([item.name for _, item in converted], ["comp_req__bits__set"])
        data = converted[0][1].to_json()
        self.assertEqual(data["tag"], "req comp_req__bits__set@2")
        self.assertEqual(data["framework"], "Sphinx-Needs")
        self.assertEqual(data["kind"], "CompReq")
        self.assertEqual(data["text"], "The component shall set *bits*.")
        self.assertEqual(data["status"], "valid")
        self.assertEqual(data["refs"], ["req feat_req__bits@3", "req aou_req__bits__ints"])
        self.assertEqual(data["location"]["file"], "requirements/index")
        self.assertEqual(data["location"]["line"], 33)

    def test_description_falls_back_to_title(self):
        need = dict(NEEDS["comp_req__bits__set"], content="")
        needs = _write(self.tmp.name, "needs.json", _needs_json({need["id"]: need}))
        self.assertEqual(convert([needs], self.config)[0][1].text, "Set bits")

    def test_single_version_is_used_without_current_version(self):
        data = {"versions": {"": {"needs": NEEDS}}}
        needs = _write(self.tmp.name, "needs.json", data)
        self.assertEqual(len(convert([needs], self.config)), 1)

    def test_duplicate_need_in_two_files_is_an_error(self):
        first = _write(self.tmp.name, "a.json", _needs_json(NEEDS))
        second = _write(self.tmp.name, "b.json", _needs_json(NEEDS))
        with self.assertRaises(ConfigError):
            convert([first, second], self.config)

    def test_unknown_rule_key_is_an_error(self):
        path = _write(self.tmp.name, "bad.yaml", CONFIG + "    record-type: CompReq\n")
        with self.assertRaises(ConfigError):
            load_config(path)

    def test_id_pattern_selects_needs(self):
        needs = dict(NEEDS)
        needs["comp_req__other__get"] = dict(NEEDS["comp_req__bits__set"], id="comp_req__other__get")
        path = _write(self.tmp.name, "needs.json", _needs_json(needs))
        config = load_config(_write(self.tmp.name, "pattern.yaml", CONFIG + "    id-pattern: ^comp_req__bits__\n"))
        self.assertEqual([item.name for _, item in convert([path], config)], ["comp_req__bits__set"])


class MainTest(unittest.TestCase):
    def test_writes_lobster_file_and_rst_page(self):
        with tempfile.TemporaryDirectory() as tmp:
            config = _write(tmp, "config.yaml", CONFIG)
            needs = _write(tmp, "needs.json", _needs_json(NEEDS))
            out = os.path.join(tmp, "out.lobster")
            rst = os.path.join(tmp, "out.rst")

            self.assertEqual(
                main([needs, "--config", config, "--out", out, "--rst-out", rst, "--title", "Component Requirements"]),
                0,
            )

            items = {}
            lobster_read(Message_Handler(), out, "level", items)
            self.assertEqual(list(items), ["req comp_req__bits__set"])
            with open(rst, encoding="utf-8") as f:
                page = f.read()
            self.assertTrue(page.startswith("Component Requirements\n======================\n"))
            self.assertIn("``comp_req__bits__set``", page)
            self.assertIn(r"The component shall set \*bits\*.", page)

    def test_invalid_config_returns_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            config = _write(tmp, "config.yaml", "conversion-rules:\n  - kind: CompReq\n")
            needs = _write(tmp, "needs.json", _needs_json(NEEDS))
            self.assertEqual(main([needs, "--config", config, "--out", os.path.join(tmp, "out.lobster")]), 1)


if __name__ == "__main__":
    unittest.main()
