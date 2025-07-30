# *******************************************************************************
# Copyright (c) 2024 Contributors to the Eclipse Foundation
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

load("@rules_java//java:java_binary.bzl", "java_binary")
load("@score_tooling//dash/tool/formatters:dash_format_converter.bzl", "dash_format_converter")

def dash_license_checker(
        visibility,
        src,
        file_type = "",
        project_config = None):
    """
    Defines a Bazel macro for creating a `java_binary` target that integrates the DASH license checker.
    The generated target is always named 'license_check', and an alias 'license-check' is automatically
    created pointing to it.

    Usage (Explicit mode):
      Provide the 'src' explicitly.
      
      If 'file_type' is omitted (empty string) and a project_config is provided, the file_type is
      determined from the "source_code" field in project_config.
      
      Example project_config (project_config.bzl):
      
          PROJECT_CONFIG = {
              "asil_level": "QM",
              "source_code": ["python"]
          }
      
      - If "python" is present, file_type becomes "requirements".
      - If "rust" is present, file_type becomes "cargo".
    
    Example usage:
    
         dash_license_checker(
             visibility = ["//visibility:public"],
             src = "//docs:requirements",  # a filegroup target pointing to your requirements.txt file
             file_type = "",                # omitted so that it's determined from project_config
             project_config = PROJECT_CONFIG
         )
    """
    name = "license_check"

    # Determine file_type from project_config if not provided.
    if file_type == "":
        if project_config != None:
            languages = project_config.get("source_code", [])
            if "python" in languages:
                file_type = "requirements"
            elif "rust" in languages:
                file_type = "cargo"
            else:
                fail("Unsupported project type in project_config: {}".format(languages))
        else:
            fail("file_type is not provided and no project_config was passed to determine it.")

    if src == "":
        fail("You must provide the src explicitly.")

    # Convert the dependency file using the provided file_type.
    dash_format_converter(
        name = "{}2dash".format(name),
        requirement_file = src,
        file_type = file_type,
    )

    # Create the java_binary target that runs the license checker.
    java_binary(
        name = "license.check.{}".format(name),
        main_class = "org.eclipse.dash.licenses.cli.Main",
        runtime_deps = [
            "@score_tooling//dash:jar",
        ],
        # We'll build up "args" in the order: [ static options ] + [ file last ]
        args = [
                   # If we have any always-on flags, put them here
               ] +
               [
                   # The file is last
                   "$(location :{}2dash)".format(name),
               ],
        data = [
            ":{}2dash".format(name),
        ],
        visibility = ["//visibility:public"],
    )

    # Automatically create an alias "license-check" that points to the java_binary target.
    native.alias(
        name = "license-check",
        actual = ":license.check.{}".format(name),
        visibility = visibility,
    )
