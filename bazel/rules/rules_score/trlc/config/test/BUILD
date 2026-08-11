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

load("@trlc//:trlc.bzl", "trlc_requirements")
load(":trlc_check_test.bzl", "trlc_check_test")

# ==============================================================================
# ASIL Safety Check — Test Fixtures
#
# Scenario matrix:
#
#   Upstream ASIL  |  Downstream ASIL  |  Result
#   ---------------+-------------------+--------
#   QM             |  QM               |  PASS  (same level)
#   B              |  B                |  PASS  (same level)
#   B              |  D                |  PASS  (raised level)
#   D              |  D                |  PASS  (same level)
#   B              |  QM               |  FAIL  (degraded — check fires)
#   D              |  B                |  FAIL  (degraded — check fires)
#   D              |  QM               |  FAIL  (degraded — check fires)
#
# ==============================================================================

# ------------------------------------------------------------------------------
# Shared upstream requirements (ASRs at QM / B / D)
# ------------------------------------------------------------------------------

trlc_requirements(
    name = "valid_asr",
    srcs = ["valid_asr.trlc"],
    spec = ["//bazel/rules/rules_score/trlc/config:score_requirements_model"],
    visibility = ["//visibility:private"],
)

# ------------------------------------------------------------------------------
# Valid scenarios — ASIL level is maintained or raised downstream
# bazel test //bazel/rules/rules_score/trlc/config/test:asil_check_valid_test
# Expected result: PASS
# ------------------------------------------------------------------------------

trlc_requirements(
    name = "valid_feat_req",
    srcs = ["valid_feat_req.trlc"],
    spec = ["//bazel/rules/rules_score/trlc/config:score_requirements_model"],
    deps = [":valid_asr"],
    visibility = ["//visibility:private"],
)

trlc_requirements(
    name = "valid_comp_req",
    srcs = ["valid_comp_req.trlc"],
    spec = ["//bazel/rules/rules_score/trlc/config:score_requirements_model"],
    deps = [
        ":valid_asr",
        ":valid_feat_req",
    ],
    visibility = ["//visibility:private"],
)

# Only the leaf target is passed — its depset transitively includes valid_feat_req
# and valid_asr (via deps), so the RSL spec file is passed to TRLC exactly once.
trlc_check_test(
    name = "asil_check_valid_test",
    reqs = [":valid_comp_req"],
)

# ------------------------------------------------------------------------------
# Invalid scenarios — ASIL level is degraded downstream (check violations)
#
# These targets are tagged "manual" because trlc_requirements_test will FAIL
# as expected when TRLC reports the ASIL check violation as an error.
# Run them manually to observe the error output:
#
#   bazel test //bazel/rules/rules_score/trlc/config/test:asil_check_invalid_feat_test
#   bazel test //bazel/rules/rules_score/trlc/config/test:asil_check_invalid_comp_test
# ------------------------------------------------------------------------------

trlc_requirements(
    name = "invalid_feat_req",
    srcs = ["invalid_feat_req.trlc"],
    spec = ["//bazel/rules/rules_score/trlc/config:score_requirements_model"],
    deps = [":valid_asr"],
    tags = ["manual"],
    visibility = ["//visibility:private"],
)

# Expected result: FAIL — 3 ASIL check violations (FEAT_INVALID_001/002/003)
# invalid_feat_req deps on valid_asr, so all needed files are included transitively.
trlc_check_test(
    name = "asil_check_invalid_feat_test",
    reqs = [":invalid_feat_req"],
    tags = ["manual"],
)

trlc_requirements(
    name = "invalid_comp_req",
    srcs = ["invalid_comp_req.trlc"],
    spec = ["//bazel/rules/rules_score/trlc/config:score_requirements_model"],
    deps = [
        ":valid_asr",
        ":valid_feat_req",
    ],
    tags = ["manual"],
    visibility = ["//visibility:private"],
)

# Expected result: FAIL — 4 ASIL check violations (COMP_INVALID_001/002/003/004)
# invalid_comp_req deps on valid_asr and valid_feat_req, so all needed files are included transitively.
trlc_check_test(
    name = "asil_check_invalid_comp_test",
    reqs = [":invalid_comp_req"],
    tags = ["manual"],
)
