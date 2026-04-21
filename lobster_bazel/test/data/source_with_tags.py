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
from dependency_with_tags import module_function
from trandependency_with_tags import library_function


def process_safety_function():
    # req-traceability: COMP_REQ_001
    pass


def validate_input():
    # req-traceability: FEAT_REQ_042
    pass


def validate_output():
    # req-traceability: FEAT_REQ_042
    module_function()


# req-traceability: FEAT_REQ_043
def another_safety_function():
    library_function()
