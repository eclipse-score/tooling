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

def github_urls(path):
    return [
        "https://github.com/" + path,
    ]

def _lobster_repository_impl(repository_ctx):
    _VERSION = "0.14.1"
    _PATH = "bmw-software-engineering/lobster/archive/refs/tags/lobster-{version}.tar.gz".format(version = _VERSION)
    repository_ctx.download_and_extract(
        url = github_urls(_PATH),
        sha256 = "5a0b86c62cadc872dcb1b79485ba72953400bcdc42f97c5b5aefe210e92ce6ff",
        stripPrefix = "lobster-lobster-{version}".format(version = _VERSION),
    )

lobster_repository = repository_rule(
    implementation = _lobster_repository_impl,
)

def _lobster_impl(ctx):
    lobster_repository(name = "lobster")

lobster_ext = module_extension(
    implementation = _lobster_impl,
)
