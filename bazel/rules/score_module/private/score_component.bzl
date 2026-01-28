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
Safety Element out of Context (SEooC) build rules for S-CORE projects.

This module provides macros and rules for building SEooC documentation modules
following S-CORE process guidelines. A SEooC is a safety-related element developed
independently of a specific vehicle project.
"""

load("//bazel/rules/score_module/private:providers.bzl", "SphinxSourcesInfo")
load("//bazel/rules/score_module/private:sphinx_module.bzl", "sphinx_module")

# ============================================================================
# Private Rule Implementation
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

# ============================================================================
# Path Computation and Validation Helpers
# ============================================================================

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

# ============================================================================
# Artifact Processing Functions
# ============================================================================

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

def _software_component_index_impl(ctx):
    """Generate index.rst file with references to all SEooC artifacts.

    This rule creates a Sphinx index.rst file that includes references to all
    the SEooC documentation artifacts (assumptions of use, requirements, design,
    and safety analysis).

    Args:
        ctx: Rule context

    Returns:
        DefaultInfo provider with generated index.rst file
    """

    # Declare output index file
    index_rst = ctx.actions.declare_file(ctx.label.name + "/index.rst")
    output_files = [index_rst]

    # Define artifact types to process
    artifact_types = [
        "assumptions_of_use",
        "component_requirements",
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
    # The HTML merger will handle copying the actual HTML files
    # We generate RST links that will work once HTML is merged
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
            "{component_requirements}": "\n   ".join(artifacts_by_type["component_requirements"]),
            "{architectural_design}": "\n   ".join(artifacts_by_type["architectural_design"]),
            "{dependability_analysis}": "\n   ".join(artifacts_by_type["dependability_analysis"]),
            "{checklists}": "\n   ".join(artifacts_by_type["checklists"]),
            "{submodules}": deps_links,
        },
    )

    return [
        DefaultInfo(files = depset(output_files)),
    ]

# ============================================================================
# Private Rule Definition
# ============================================================================

_software_component_index = rule(
    implementation = _software_component_index_impl,
    doc = "Generates index.rst file with references to SEooC artifacts",
    attrs = {
        "module_name": attr.string(
            mandatory = True,
            doc = "Name of the SEooC module (used as document title)",
        ),
        "description": attr.string(
            mandatory = True,
            doc = "Description of the SEooC component that appears at the beginning of the documentation. Supports RST formatting.",
        ),
        "assumptions_of_use": attr.label_list(
            mandatory = True,
            doc = "Assumptions of Use targets or files as defined in the S-CORE process. Can be assumptions_of_use targets or raw .rst/.md files.",
        ),
        "component_requirements": attr.label_list(
            mandatory = True,
            doc = "Component requirements targets or files as defined in the S-CORE process. Can be component_requirements targets or raw .rst/.md files.",
        ),
        "architectural_design": attr.label_list(
            mandatory = True,
            doc = "Architectural design targets or files as defined in the S-CORE process. Can be architectural_design targets or raw .rst/.md files.",
        ),
        "dependability_analysis": attr.label_list(
            mandatory = True,
            doc = "Dependability analysis targets or files as defined in the S-CORE process. Can be dependability_analysis targets or raw .rst/.md files.",
        ),
        "checklists": attr.label_list(
            mandatory = True,
            doc = "Safety checklists targets or files as defined in the S-CORE process.",
        ),
        "template": attr.label(
            allow_single_file = [".rst"],
            mandatory = True,
            doc = "Template file for generating index.rst",
        ),
        "deps": attr.label_list(
            default = [],
            doc = "Dependencies on other score_component modules (submodules). Their index.rst files will be linked in the Submodules section.",
        ),
    },
)

# ============================================================================
# Public Macro
# ============================================================================

def score_component(
        name,
        assumptions_of_use,
        component_requirements,
        architectural_design,
        dependability_analysis,
        description,
        checklists = [],
        implementations = [],
        tests = [],
        deps = [],
        sphinx = "//bazel/rules/score_module:score_build",
        visibility = None):
    """Define a Safety Element out of Context (SEooC) following S-CORE process guidelines.

    This macro creates a complete SEooC module with integrated documentation
    generation. It generates an index.rst file referencing all SEooC artifacts
    and builds HTML documentation using the sphinx_module infrastructure.

    A SEooC is a safety-related architectural element (e.g., a software component)
    that is developed independently of a specific vehicle project and can be
    integrated into different vehicle platforms.

    Args:
        name: The name of the safety element module. Used as the base name for
            all generated targets.
        assumptions_of_use: List of labels to assumptions_of_use targets or raw
            .rst/.md files containing the Assumptions of Use, which define the
            safety-relevant operating conditions and constraints for the SEooC
            as defined in the S-CORE process.
        component_requirements: List of labels to component_requirements targets
            or raw .rst/.md files containing the component requirements specification,
            defining functional and safety requirements as defined in the S-CORE process.
        architectural_design: List of labels to architectural_design targets or raw
            .rst/.md files containing the architectural design specification, describing
            the software architecture and design decisions as defined in the S-CORE process.
        dependability_analysis: List of labels to dependability_analysis targets or raw
            .rst/.md files containing the safety analysis, including FMEA, FMEDA, FTA,
            or other safety analysis results as defined in the S-CORE process.
        description: String containing a high-level description of the SEooC
            component. This text appears at the beginning of the generated documentation,
            providing context about what the component does and its purpose.
            Supports RST formatting.
        checklists: Optional list of labels to .rst or .md files containing
            safety checklists and verification documents as defined in the
            S-CORE process.
        implementations: Optional list of labels to Bazel targets representing
            the actual software implementation (cc_library, cc_binary, etc.)
            that realizes the component requirements. This is the source code
            that implements the safety functions as defined in the S-CORE process.
        tests: Optional list of labels to Bazel test targets (cc_test, py_test, etc.)
            that verify the implementation against requirements. Includes unit
            tests and integration tests as defined in the S-CORE process.
        deps: Optional list of other sphinx_module or SEooC targets this module
            depends on. Cross-references will work automatically.
        sphinx: Label to sphinx build binary. Default: //bazel/rules/score_module:score_build
        visibility: Bazel visibility specification for the generated SEooC targets.

    Generated Targets:
        <name>_seooc_index: Internal rule that generates index.rst and copies artifacts
        <name>: Main SEooC target (sphinx_module) with HTML documentation
        <name>_needs: Internal target for sphinx-needs JSON generation
    """

    # Step 1: Generate index.rst and collect all artifacts
    _software_component_index(
        name = name + "_seooc_index",
        module_name = name,
        description = description,
        template = Label("//bazel/rules/score_module:templates/seooc_index.template.rst"),
        assumptions_of_use = assumptions_of_use,
        component_requirements = component_requirements,
        architectural_design = architectural_design,
        dependability_analysis = dependability_analysis,
        checklists = checklists,
        deps = deps,
        visibility = ["//visibility:private"],
    )

    # Step 2: Create sphinx_module using generated index and artifacts
    # The index file is part of the _seooc_index target outputs
    sphinx_module(
        name = name,
        srcs = [":" + name + "_seooc_index"],
        index = ":" + name + "_seooc_index",  # Label to the target, not a path
        deps = deps,
        sphinx = sphinx,
        visibility = visibility,
    )
