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
"""Checks the LOBSTER items the requirement macros build from needs.json."""

import os
import unittest

from lobster.common.errors import Message_Handler
from lobster.common.io import lobster_read


def _items():
    items = {}
    for path in os.environ["LOBSTER_FILES"].split():
        lobster_read(Message_Handler(), path, "requirements", items)
    return {item.tag.tag: item for item in items.values()}


class RequirementsNeedsOutputTest(unittest.TestCase):
    def setUp(self):
        self.items = _items()

    def test_each_kind_gets_its_trlc_record_type(self):
        # feat_req__other__feature and comp_req__other__get are satisfied by another feature/component.
        kinds = {name: item.kind for name, item in self.items.items()}
        self.assertEqual(
            kinds,
            {
                "stkh_req__bits__fast_io": "AssumedSystemReq",
                "feat_req__bits__bitmanipulation": "FeatReq",
                "comp_req__bits__set": "CompReq",
                "aou_req__bits__integral": "AoU",
            },
        )

    def test_component_requirement_traces_like_lobster_trlc(self):
        item = self.items["comp_req__bits__set"]
        self.assertEqual(str(item.tag), "req comp_req__bits__set@1")
        self.assertEqual(item.framework, "Sphinx-Needs")
        self.assertEqual(
            sorted(str(ref) for ref in item.unresolved_references),
            ["req aou_req__bits__integral", "req feat_req__bits__bitmanipulation@2"],
        )

    def test_invalid_requirements_are_not_exported(self):
        self.assertNotIn("comp_req__bits__retired", self.items)


if __name__ == "__main__":
    unittest.main()
