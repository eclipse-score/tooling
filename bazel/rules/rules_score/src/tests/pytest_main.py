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
"""Runs pytest on the test files passed as arguments and writes the JUnit XML Bazel expects."""

import os
import sys

import pytest

if __name__ == "__main__":
    pytest_args = ["-p", "no:cacheprovider", "-o", "junit_family=xunit1"]
    xml_output = os.environ.get("XML_OUTPUT_FILE")
    if xml_output:
        pytest_args.append(f"--junitxml={xml_output}")
    sys.exit(pytest.main(pytest_args + sys.argv[1:]))
