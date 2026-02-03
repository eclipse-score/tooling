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

"""
Dependable Element build rules for S-CORE projects.

This module provides macros and rules for defining dependable elements (Safety
Elements out of Context - SEooC) following S-CORE process guidelines. A dependable
element is a safety-critical component with comprehensive documentation including
assumptions of use, requirements, design, and safety analysis.
"""

load(
    "//bazel/rules/rules_score:providers.bzl",
    "DependableElementInfo",
    "SphinxSourcesInfo",
)
load("//bazel/rules/rules_score/private:sphinx_module.bzl", "sphinx_module")

# ============================================================================
# Helper Functions for Documentation Generation
# ============================================================================

def _get_sphinx_files(target):
    return target[SphinxSourcesInfo].srcs.to_list()

def _filter_doc_files(files):
    """Filter files to only include documentation files.

    Args:
        files: List of files to filter

    Returns:
        List of documentation files
    """
    return [f for f in files if f.extension in ["rst", "md", "puml", "plantuml", "png", "svg"]]

def _find_common_directory(files):
    """Find the longest common directory path for a list of files.

    Args:
        files: List of File objects

    Returns:
        String representing the common directory path, or empty string if none
    """
    if not files:
        return ""

    # Get all directory paths
    dirs = [f.dirname for f in files]

    if not dirs:
        return ""

    # Start with first directory
    common = dirs[0]

    # Iterate through all directories to find common prefix
    for d in dirs[1:]:
        # Find common prefix between common and d
        # Split into path components
        common_parts = common.split("/")
        d_parts = d.split("/")

        # Find matching prefix
        new_common_parts = []
        for i in range(min(len(common_parts), len(d_parts))):
            if common_parts[i] == d_parts[i]:
                new_common_parts.append(common_parts[i])
            else:
                break

        common = "/".join(new_common_parts)

        if not common:
            break

    return common

def _compute_relative_path(file, common_dir):
    """Compute relative path from common directory to file.

    Args:
        file: File object
        common_dir: Common directory path string

    Returns:
        String containing the relative path
    """
    file_dir = file.dirname

    if not common_dir:
        return file.basename

    if not file_dir.startswith(common_dir):
        return file.basename

    if file_dir == common_dir:
        return file.basename

    relative_subdir = file_dir[len(common_dir):].lstrip("/")
    return relative_subdir + "/" + file.basename

def _is_document_file(file):
    """Check if file should be included in toctree.

    Args:
        file: File object

    Returns:
        Boolean indicating if file is a document (.rst or .md)
    """
    return file.extension in ["rst", "md"]

def _create_artifact_symlink(ctx, artifact_name, artifact_file, relative_path):
    """Create symlink for artifact file in output directory.

    Args:
        ctx: Rule context
        artifact_name: Name of artifact type (e.g., "architectural_design")
        artifact_file: Source file
        relative_path: Relative path within artifact directory

    Returns:
        Declared output file
    """
    output_file = ctx.actions.declare_file(
        ctx.label.name + "/" + artifact_name + "/" + relative_path,
    )

    ctx.actions.symlink(
        output = output_file,
        target_file = artifact_file,
    )

    return output_file

def _process_artifact_files(ctx, artifact_name, label):
    """Process all files from a single label for a given artifact type.

    Args:
        ctx: Rule context
        artifact_name: Name of artifact type
        label: Label to process

    Returns:
        Tuple of (output_files, index_references)
    """
    output_files = []
    index_refs = []

    # Get and filter files
    all_files = _get_sphinx_files(label)
    doc_files = _filter_doc_files(all_files)

    if not doc_files:
        return (output_files, index_refs)

    # Find common directory to preserve hierarchy
    common_dir = _find_common_directory(doc_files)

    # Process each file
    for artifact_file in doc_files:
        # Compute paths
        relative_path = _compute_relative_path(artifact_file, common_dir)

        # Create symlink
        output_file = _create_artifact_symlink(
            ctx,
            artifact_name,
            artifact_file,
            relative_path,
        )
        output_files.append(output_file)

        # Add to index if it's a document file
        if _is_document_file(artifact_file):
            doc_ref = (artifact_name + "/" + relative_path) \
                .replace(".rst", "") \
                .replace(".md", "")
            index_refs.append(doc_ref)

    return (output_files, index_refs)

def _process_artifact_type(ctx, artifact_name):
    """Process all labels for a given artifact type.

    Args:
        ctx: Rule context
        artifact_name: Name of artifact type (e.g., "architectural_design")

    Returns:
        Tuple of (output_files, index_references)
    """
    output_files = []
    index_refs = []

    attr_list = getattr(ctx.attr, artifact_name)
    if not attr_list:
        return (output_files, index_refs)

    # Process each label
    for label in attr_list:
        label_outputs, label_refs = _process_artifact_files(
            ctx,
            artifact_name,
            label,
        )
        output_files.extend(label_outputs)
        index_refs.extend(label_refs)

    return (output_files, index_refs)

def _process_deps(ctx):
    """Process deps to generate references to submodule documentation.

    The HTML merger in sphinx_module will copy the HTML directories from deps.
    We generate RST bullet list with links to those HTML directories.

    Args:
        ctx: Rule context

    Returns:
        String containing RST-formatted bullet list of links
    """
    if not ctx.attr.deps:
        return ""

    # Generate RST bullet list with links to submodule HTML
    links = []
    for dep in ctx.attr.deps:
        dep_name = dep.label.name

        # Create a link to the index.html that will be merged
        # Format: * `Module Name <module_name/index.html>`_
        # Use underscores in name for readability, convert to spaces for display
        display_name = dep_name.replace("_", " ").title()
        links.append("* `{} <{}/index.html>`_".format(display_name, dep_name))

    return "\n".join(links)

# ============================================================================
# Index Generation Rule Implementation
# ============================================================================

def _dependable_element_index_impl(ctx):
    """Generate index.rst file with references to all dependable element artifacts.

    This rule creates a Sphinx index.rst file that includes references to all
    the documentation artifacts for the dependable element.

    Args:
        ctx: Rule context

    Returns:
        DefaultInfo provider with generated index.rst file
    """

    # Declare output index file
    index_rst = ctx.actions.declare_file(ctx.label.name + "/index.rst")
    output_files = [index_rst]

    # Define artifact types to process
    # Note: "requirements" can contain both component_requirements and feature_requirements
    artifact_types = [
        "assumptions_of_use",
        "requirements",
        "architectural_design",
        "dependability_analysis",
        "checklists",
    ]

    # Process each artifact type
    artifacts_by_type = {}
    for artifact_name in artifact_types:
        files, refs = _process_artifact_type(ctx, artifact_name)
        output_files.extend(files)
        artifacts_by_type[artifact_name] = refs

    # Process dependencies (submodules)
    deps_links = _process_deps(ctx)

    # Generate index file from template
    title = ctx.attr.module_name
    underline = "=" * len(title)

    ctx.actions.expand_template(
        template = ctx.file.template,
        output = index_rst,
        substitutions = {
            "{title}": title,
            "{underline}": underline,
            "{description}": ctx.attr.description,
            "{assumptions_of_use}": "\n   ".join(artifacts_by_type["assumptions_of_use"]),
            "{component_requirements}": "\n   ".join(artifacts_by_type["requirements"]),
            "{architectural_design}": "\n   ".join(artifacts_by_type["architectural_design"]),
            "{dependability_analysis}": "\n   ".join(artifacts_by_type["dependability_analysis"]),
            "{checklists}": "\n   ".join(artifacts_by_type["checklists"]),
            "{submodules}": deps_links,
        },
    )

    return [
        DefaultInfo(files = depset(output_files)),
    ]

_dependable_element_index = rule(
    implementation = _dependable_element_index_impl,
    doc = "Generates index.rst file with references to dependable element artifacts",
    attrs = {
        "module_name": attr.string(
            mandatory = True,
            doc = "Name of the dependable element module (used as document title)",
        ),
        "description": attr.string(
            mandatory = True,
            doc = "Description of the dependable element. Supports RST formatting.",
        ),
        "assumptions_of_use": attr.label_list(
            mandatory = True,
            doc = "Assumptions of Use targets or files.",
        ),
        "requirements": attr.label_list(
            mandatory = True,
            doc = "Requirements targets (component_requirements, feature_requirements, etc.).",
        ),
        "architectural_design": attr.label_list(
            mandatory = True,
            doc = "Architectural design targets or files.",
        ),
        "dependability_analysis": attr.label_list(
            mandatory = True,
            doc = "Dependability analysis targets or files.",
        ),
        "checklists": attr.label_list(
            default = [],
            doc = "Safety checklists targets or files.",
        ),
        "template": attr.label(
            allow_single_file = [".rst"],
            mandatory = True,
            doc = "Template file for generating index.rst",
        ),
        "deps": attr.label_list(
            default = [],
            doc = "Dependencies on other dependable element modules (submodules).",
        ),
    },
)

# ============================================================================
# Provider Rule Implementation
# ============================================================================

def _dependable_element_provider_impl(ctx):
    """Provide DependableElementInfo for a dependable element.

    This rule collects metadata about the dependable element and provides
    it through the DependableElementInfo provider.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DependableElementInfo and SphinxSourcesInfo
    """

    # Collect depsets for each artifact type
    assumptions_depset = depset(ctx.files.assumptions_of_use)
    requirements_depset = depset(ctx.files.requirements)
    arch_design_depset = depset(ctx.files.architectural_design)
    dep_analysis_depset = depset(ctx.files.dependability_analysis)
    components_depset = depset(ctx.attr.components)
    tests_depset = depset(ctx.attr.tests)

    # Collect all source files for Sphinx
    all_files = depset(
        direct = ctx.files.assumptions_of_use +
                 ctx.files.requirements +
                 ctx.files.architectural_design +
                 ctx.files.dependability_analysis,
    )

    return [
        DependableElementInfo(
            name = ctx.label.name,
            description = ctx.attr.description,
            assumptions_of_use = assumptions_depset,
            requirements = requirements_depset,
            architectural_design = arch_design_depset,
            dependability_analysis = dep_analysis_depset,
            consists_of = components_depset,
            tests = tests_depset,
        ),
        SphinxSourcesInfo(
            srcs = all_files,
            transitive_srcs = all_files,
        ),
    ]

_dependable_element_provider = rule(
    implementation = _dependable_element_provider_impl,
    doc = "Provider rule for dependable element metadata",
    attrs = {
        "description": attr.string(
            mandatory = True,
            doc = "Description of the dependable element",
        ),
        "assumptions_of_use": attr.label_list(
            mandatory = True,
            doc = "Assumptions of Use targets or files",
        ),
        "requirements": attr.label_list(
            mandatory = True,
            doc = "Requirements targets",
        ),
        "architectural_design": attr.label_list(
            mandatory = True,
            doc = "Architectural design targets or files",
        ),
        "dependability_analysis": attr.label_list(
            mandatory = True,
            doc = "Dependability analysis targets or files",
        ),
        "components": attr.label_list(
            default = [],
            doc = "Component and/or unit targets that comprise this element",
        ),
        "tests": attr.label_list(
            default = [],
            doc = "Test targets",
        ),
    },
)

# ============================================================================
# Public Macro
# ============================================================================

def dependable_element(
        name,
        description,
        assumptions_of_use,
        requirements,
        architectural_design,
        dependability_analysis,
        components,
        tests,
        checklists = [],
        deps = [],
        sphinx = Label("@score_tooling//bazel/rules/rules_score:score_build"),
        testonly = True,
        visibility = None):
    """Define a dependable element (Safety Element out of Context - SEooC) following S-CORE process guidelines.

    This macro creates a complete dependable element with integrated documentation
    generation. It generates an index.rst file referencing all artifacts and builds
    HTML documentation using the sphinx_module infrastructure.

    A dependable element is a safety-critical component that can be developed
    independently and integrated into different systems. It includes comprehensive
    documentation covering all aspects required for safety certification.

    Args:
        name: The name of the dependable element. Used as the base name for
            all generated targets.
        description: String containing a high-level description of the element.
            This text provides context about what the element does and its purpose.
            Supports RST formatting.
        assumptions_of_use: List of labels to assumptions_of_use targets that
            define the safety-relevant operating conditions and constraints.
        requirements: List of labels to requirements targets (component_requirements,
            feature_requirements, etc.) that define functional and safety requirements.
        architectural_design: List of labels to architectural_design targets that
            describe the software architecture and design decisions.
        dependability_analysis: List of labels to dependability_analysis targets
            containing safety analysis results (FMEA, FMEDA, FTA, DFA, etc.).
        components: List of labels to component and/or unit targets that implement
            this dependable element.
        tests: List of labels to Bazel test targets that verify the dependable
            element at the system level (integration tests, system tests).
        checklists: Optional list of labels to .rst or .md files containing
            safety checklists and verification documents.
        deps: Optional list of other module targets this element depends on.
            Cross-references will work automatically.
        sphinx: Label to sphinx build binary. Default: //bazel/rules/rules_score:score_build
        testonly: If True, only testonly targets can depend on this target.
        visibility: Bazel visibility specification for the dependable element target.

    Generated Targets:
        <name>_provider: Internal metadata provider with DependableElementInfo
        <name>_index: Internal rule that generates index.rst and copies artifacts
        <name>: Main dependable element target (sphinx_module) with HTML documentation
        <name>_needs: Internal target for sphinx-needs JSON generation

    Example:
        ```python
        dependable_element(
            name = "persistency_kvs",
            description = '''
            The Key-Value Store (KVS) component provides persistent storage capabilities
            for safety-critical applications.
            ''',
            assumptions_of_use = [":kvs_assumptions_of_use"],
            requirements = [":kvs_component_requirements"],
            architectural_design = [":kvs_architectural_design"],
            dependability_analysis = [":kvs_dependability_analysis"],
            components = [":kvs_component", ":kvs_unit"],
            tests = ["//persistency/kvs/tests:score_kvs_integration_tests"],
            deps = [
                "@score_process//:score_process_module",
                "@score_platform//:score_platform_module",
            ],
            visibility = ["//visibility:public"],
        )
        ```
    """

    # Step 1: Create provider target with DependableElementInfo
    _dependable_element_provider(
        name = name + "_provider",
        description = description,
        assumptions_of_use = assumptions_of_use,
        requirements = requirements,
        architectural_design = architectural_design,
        dependability_analysis = dependability_analysis,
        components = components,
        tests = tests,
        testonly = testonly,
        visibility = ["//visibility:private"],
    )

    # Step 2: Generate index.rst and collect all artifacts
    _dependable_element_index(
        name = name + "_index",
        module_name = name,
        description = description,
        template = Label("//bazel/rules/rules_score:templates/seooc_index.template.rst"),
        assumptions_of_use = assumptions_of_use,
        requirements = requirements,
        architectural_design = architectural_design,
        dependability_analysis = dependability_analysis,
        checklists = checklists,
        deps = deps,
        testonly = testonly,
        visibility = ["//visibility:private"],
    )

    # Step 3: Create sphinx_module using generated index and artifacts
    sphinx_module(
        name = name,
        srcs = [":" + name + "_index"],
        index = ":" + name + "_index",
        deps = deps,
        sphinx = sphinx,
        testonly = testonly,
        visibility = visibility,
    )
