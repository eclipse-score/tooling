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
"""Write the tests of a JSON spec as the gtest XML that lobster-gtest reads.

The spec is a list of objects with ``name``, ``file``, ``refs`` (requirement
ids), ``executed`` and ``failure`` (message or null).
"""

import argparse
import json
import xml.etree.ElementTree as ET


def build_xml(cases: list[dict]) -> ET.ElementTree:
    root = ET.Element("testsuites")
    suite = ET.SubElement(root, "testsuite", name="bazel")
    for case in cases:
        element = ET.SubElement(
            suite,
            "testcase",
            name=case["name"],
            file=case["file"],
            status="run" if case["executed"] else "not run",
        )
        properties = ET.SubElement(element, "properties")
        for ref in case["refs"]:
            ET.SubElement(properties, "property", name="lobster-tracing", value=ref)
        if case["failure"] is not None:
            ET.SubElement(element, "failure", message=case["failure"])
    return ET.ElementTree(root)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spec", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    with open(args.spec, encoding="utf-8") as spec:
        tree = build_xml(json.load(spec))
    tree.write(args.out, encoding="utf-8", xml_declaration=True)


if __name__ == "__main__":
    main()
