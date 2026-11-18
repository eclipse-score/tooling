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
"""Runs trlc on the given files and expects exactly N check errors with a message."""

import argparse
import contextlib
import io
import sys

from trlc import trlc


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--message", required=True)
    parser.add_argument("--count", type=int, required=True)
    parser.add_argument("files", nargs="+")
    args = parser.parse_args()

    sys.argv = ["trlc", *args.files]
    output = io.StringIO()
    with contextlib.redirect_stdout(output):
        exit_code = trlc.main()
    report = output.getvalue()

    expected = f"check error: {args.message}"
    found = report.count(expected)

    if exit_code == 0 or found != args.count:
        print(report)
        print(
            f"expected a non-zero exit code and {args.count} x '{expected}', "
            f"got exit code {exit_code} and {found} matches",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
