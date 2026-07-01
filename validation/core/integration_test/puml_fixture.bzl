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

"""Helpers for exposing PlantUML FlatBuffers fixtures in integration tests."""

load(
    "//bazel/rules/rules_score:providers.bzl",
    "ArchitecturalDesignInfo",
    "UnitDesignInfo",
    "UnitInfo",
)

def _collect_fbs_files(deps):
    files_by_category = {
        "component": [],
        "internal_api": [],
        "sequence": [],
        "unit_design_class": [],
        "unit_design_sequence": [],
        "unit_implementation_class": [],
        "unit_implementation_sequence": [],
    }

    for dep in deps:
        if ArchitecturalDesignInfo in dep:
            component_files = dep[ArchitecturalDesignInfo].static.to_list()
            internal_api_files = dep[ArchitecturalDesignInfo].internal_api.to_list()
            sequence_files = dep[ArchitecturalDesignInfo].dynamic.to_list()

            files_by_category["component"].extend(component_files)
            files_by_category["internal_api"].extend(internal_api_files)
            files_by_category["sequence"].extend(sequence_files)

        if UnitDesignInfo in dep:
            class_files = dep[UnitDesignInfo].static.to_list()
            unit_dynamic_files = dep[UnitDesignInfo].dynamic.to_list()

            files_by_category["unit_design_class"].extend(class_files)
            files_by_category["unit_design_sequence"].extend(unit_dynamic_files)

        if UnitInfo in dep:
            files_by_category["unit_implementation_class"].extend(
                dep[UnitInfo].implementation_class_fbs.to_list(),
            )
            files_by_category["unit_implementation_sequence"].extend(
                dep[UnitInfo].implementation_sequence_fbs.to_list(),
            )

    return files_by_category

def _provider_fbs_fixture_bundle_impl(ctx):
    files = _collect_fbs_files(ctx.attr.deps)

    generated = []

    def _join_path(directory, basename):
        if directory:
            return "{}/{}".format(directory, basename)
        return basename

    def _materialize_category(category):
        files_by_basename = {}
        for file in files[category]:
            if file.basename in files_by_basename:
                fail("duplicate basename {} found in {} files for {}".format(file.basename, category, [dep.label for dep in ctx.attr.deps]))
            files_by_basename[file.basename] = file

        basenames = sorted(files_by_basename.keys())
        out_dir = _join_path(ctx.attr.output_root, category)

        for basename in basenames:
            out = ctx.actions.declare_file(_join_path(out_dir, basename))

            ctx.actions.symlink(
                output = out,
                target_file = files_by_basename[basename],
            )

            generated.append(out)

    _materialize_category("component")
    _materialize_category("internal_api")
    _materialize_category("sequence")
    _materialize_category("unit_design_class")
    _materialize_category("unit_design_sequence")
    _materialize_category("unit_implementation_class")
    _materialize_category("unit_implementation_sequence")

    return [DefaultInfo(files = depset(generated))]

provider_fbs_fixture_bundle = rule(
    implementation = _provider_fbs_fixture_bundle_impl,
    attrs = {
        "deps": attr.label_list(
            mandatory = True,
        ),
        "output_root": attr.string(
            default = "",
        ),
    },
)
