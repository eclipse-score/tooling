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

# trlc_check_test: like trlc_requirements_test but omits --verify.
#
# TRLC 3.0.0's VCG (CVC5 backend) crashes when statically verifying check
# blocks that contain forall over union-typed tuple item fields (e.g.
# CompReqSourceId.item [FeatReq, AssumedSystemReq]).  Without --verify TRLC
# still evaluates the checks at runtime against actual requirement instances,
# which is sufficient for functional pass/fail testing.  The static VCG
# analysis can be re-enabled once the upstream TRLC bug is fixed.

def trlc_check_test(name, reqs, **kwargs):
    """Run TRLC on requirement files and evaluate user-defined checks.

    Unlike the standard trlc_requirements_test rule, this macro does NOT pass
    --verify to TRLC, so the CVC5-backed static analysis is skipped.  The
    checks defined in the RSL model are still evaluated against the TRLC
    requirement instances at runtime.

    Args:
        name: target name
        reqs: list of trlc_requirements targets to check
        **kwargs: forwarded to native.py_test (e.g. tags, visibility)
    """
    native.py_test(
        name = name,
        srcs = ["@trlc//:trlc.py"],
        main = "trlc.py",
        # No --verify: skip CVC5/VCG static analysis to avoid the 3.0.0 crash.
        args = ["$(locations %s)" % req for req in reqs],
        deps = ["@trlc//trlc:trlc"],
        data = reqs,
        **kwargs
    )
