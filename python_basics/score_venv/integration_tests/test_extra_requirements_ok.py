# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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
import os 

def test_venv_ok():
    
    runfiles = os.getenv("RUNFILES_DIR")
    packages = os.listdir(runfiles)
    assert any(x.endswith("requests") for x in packages), f"'Request not found in runfiles: {runfiles}"
    try:
        import requests
    except Exception  as e:
        assert False, f"Could not import requests. Error: {e}"
