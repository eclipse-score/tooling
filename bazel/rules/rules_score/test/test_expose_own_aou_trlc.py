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
"""Tests for expose_own_aou_trlc."""

import unittest

from expose_own_aou_trlc import DEFAULT_JUSTIFICATION, expose_own_aou_source

_SAMPLE = """\
/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 ********************************************************************************/
package MyAoUs

import ScoreReq

ScoreReq.AoU AoU1 {
    description = "First AoU."
    safety      = ScoreReq.Asil.B
    version     = 1
}

ScoreReq.AoU AoU2 {
    description = "Second AoU."
    safety      = ScoreReq.Asil.B
    version     = 1
}
"""

_HEADER_ONLY = "package Empty\n\nimport ScoreReq\n"


class TestExposeOwnAouSource(unittest.TestCase):
    """Tests for expose_own_aou_source."""

    def test_all_records_retyped_to_forwarded_aou(self) -> None:
        result = expose_own_aou_source(_SAMPLE)
        self.assertIn("ScoreReq.ReceivedAoU AoU1 {", result)
        self.assertIn("ScoreReq.ReceivedAoU AoU2 {", result)
        self.assertNotIn("ScoreReq.AoU AoU1", result)
        self.assertNotIn("ScoreReq.AoU AoU2", result)

    def test_default_justification_injected(self) -> None:
        result = expose_own_aou_source(_SAMPLE)
        self.assertEqual(result.count(f'justification = "{DEFAULT_JUSTIFICATION}"'), 2)

    def test_custom_justification_injected(self) -> None:
        result = expose_own_aou_source(_SAMPLE, justification="custom text")
        self.assertIn('justification = "custom text"', result)
        self.assertNotIn(DEFAULT_JUSTIFICATION, result)

    def test_original_fields_preserved(self) -> None:
        result = expose_own_aou_source(_SAMPLE)
        self.assertIn('description = "First AoU."', result)
        self.assertIn('description = "Second AoU."', result)
        self.assertIn("safety      = ScoreReq.Asil.B", result)
        self.assertIn("version     = 1", result)

    def test_record_order_preserved(self) -> None:
        result = expose_own_aou_source(_SAMPLE)
        self.assertLess(result.index("AoU1"), result.index("AoU2"))

    def test_header_preserved_verbatim(self) -> None:
        result = expose_own_aou_source(_SAMPLE)
        self.assertIn("package MyAoUs", result)
        self.assertIn("import ScoreReq", result)

    def test_header_only_file_with_no_records(self) -> None:
        result = expose_own_aou_source(_HEADER_ONLY)
        self.assertEqual(result, _HEADER_ONLY)

    def test_identity_unchanged_across_retype(self) -> None:
        """Package + record name must be unchanged so a derived_from
        reference written against the original AoU keeps resolving."""
        result = expose_own_aou_source(_SAMPLE)
        self.assertIn("package MyAoUs", result)
        self.assertIn("AoU1 {", result)
        self.assertIn("AoU2 {", result)

    def test_justification_is_escaped(self) -> None:
        result = expose_own_aou_source(_SAMPLE, justification='contains "quotes" and \\backslash')
        self.assertIn('justification = "contains \\"quotes\\" and \\\\backslash"', result)


if __name__ == "__main__":
    unittest.main()
