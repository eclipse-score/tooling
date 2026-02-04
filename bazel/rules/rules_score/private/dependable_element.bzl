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
    "ComponentInfo",
    "SphinxSourcesInfo",
    "UnitInfo",
)
load("//bazel/rules/rules_score/private:sphinx_module.bzl", "sphinx_module")

# ============================================================================
# Template Constants
# ============================================================================

_UNIT_DESIGN_SECTION_TEMPLATE = """Unit Design
-----------

.. toctree::
   :maxdepth: 2

{design_refs}"""

_IMPLEMENTATION_SECTION_TEMPLATE = """Implementation
--------------

This {entity_type} is implemented by the following targets:

{implementation_list}"""

_TESTS_SECTION_TEMPLATE = """Tests
-----

This {entity_type} is verified by the following test targets:

{test_list}"""

_COMPONENT_REQUIREMENTS_SECTION_TEMPLATE = """Component Requirements
----------------------

.. toctree::
   :maxdepth: 2

{requirements_refs}"""

_COMPONENT_UNITS_SECTION_TEMPLATE = """Units
-----

This component is composed of the following units:

{unit_links}"""

_UNIT_TEMPLATE = """

Unit: {unit_name}
{underline}

{design_section}{implementation_section}{tests_section}"""

_COMPONENT_TEMPLATE = """

Component: {component_name}
{underline}

{requirements_section}{units_section}{implementation_section}{tests_section}"""

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

def _get_component_names(components):
    return [c.label.name for c in components]

def _collect_units_recursive(components, visited_units = None):
    """Iteratively collect all units from components, handling nested components.

    Uses a stack-based approach to avoid Starlark recursion limitations.

    Args:
        components: List of component targets
        visited_units: Dict of unit names already visited (for deduplication)

    Returns:
        Dict mapping unit names to unit targets
    """
    if visited_units == None:
        visited_units = {}

    # Process components iteratively using a work queue approach
    # Since Starlark doesn't support while loops, we use a for loop with a large enough range
    # and track our own index
    to_process = [] + components

    for _ in range(1000):  # Max depth to prevent infinite loops
        if not to_process:
            break
        comp_target = to_process.pop(0)

        # Check if this is a component with ComponentInfo
        if ComponentInfo in comp_target:
            comp_info = comp_target[ComponentInfo]

            # Process nested components
            nested_components = comp_info.components.to_list()
            for nested in nested_components:
                # Check if nested item is a unit or component
                if UnitInfo in nested:
                    unit_name = nested.label.name
                    if unit_name not in visited_units:
                        visited_units[unit_name] = nested
                elif ComponentInfo in nested:
                    # Add nested component to queue for processing
                    to_process.append(nested)

            # Check if this is directly a unit
        elif UnitInfo in comp_target:
            unit_name = comp_target.label.name
            if unit_name not in visited_units:
                visited_units[unit_name] = comp_target

    return visited_units

def _generate_unit_doc(ctx, unit_target, unit_name):
    """Generate RST documentation for a single unit.

    Args:
        ctx: Rule context
        unit_target: The unit target
        unit_name: Name of the unit

    Returns:
        Tuple of (rst_file, list_of_output_files)
    """
    unit_info = unit_target[UnitInfo]

    # Create RST file for this unit
    unit_rst = ctx.actions.declare_file(ctx.label.name + "/units/" + unit_name + ".rst")

    # Collect design files - unit_design depset contains File objects
    design_files = []
    design_refs = []
    if unit_info.unit_design:
        doc_files = _filter_doc_files(unit_info.unit_design.to_list())

        if doc_files:
            # Find common directory
            common_dir = _find_common_directory(doc_files)

            for f in doc_files:
                relative_path = _compute_relative_path(f, common_dir)
                output_file = _create_artifact_symlink(
                    ctx,
                    "units/" + unit_name + "_design",
                    f,
                    relative_path,
                )
                design_files.append(output_file)

                if _is_document_file(f):
                    doc_ref = ("units/" + unit_name + "_design/" + relative_path) \
                        .replace(".rst", "") \
                        .replace(".md", "")
                    design_refs.append("   " + doc_ref)

    # Collect implementation target names
    impl_names = []
    if unit_info.implementation:
        for impl in unit_info.implementation.to_list():
            impl_names.append(impl.label)

    # Collect test target names
    test_names = []
    if unit_info.tests:
        for test in unit_info.tests.to_list():
            test_names.append(test.label)

    # Generate RST content using template
    underline = "=" * (len("Unit: " + unit_name))

    # Generate sections from template constants
    design_section = ""
    if design_refs:
        design_section = "\n" + _UNIT_DESIGN_SECTION_TEMPLATE.format(
            design_refs = "\n".join(design_refs),
        ) + "\n"

    implementation_section = ""
    if impl_names:
        impl_list = "\n".join(["- ``" + str(impl) + "``" for impl in impl_names])
        implementation_section = "\n" + _IMPLEMENTATION_SECTION_TEMPLATE.format(
            entity_type = "unit",
            implementation_list = impl_list,
        ) + "\n"

    tests_section = ""
    if test_names:
        test_list = "\n".join(["- ``" + str(test) + "``" for test in test_names])
        tests_section = "\n" + _TESTS_SECTION_TEMPLATE.format(
            entity_type = "unit",
            test_list = test_list,
        ) + "\n"

    # Generate unit RST content from template constant
    unit_content = _UNIT_TEMPLATE.format(
        unit_name = unit_name,
        underline = underline,
        design_section = design_section,
        implementation_section = implementation_section,
        tests_section = tests_section,
    )

    ctx.actions.write(
        output = unit_rst,
        content = unit_content,
    )

    return (unit_rst, design_files)

def _generate_component_doc(ctx, comp_target, comp_name, unit_names):
    """Generate RST documentation for a single component.

    Args:
        ctx: Rule context
        comp_target: The component target
        comp_name: Name of the component
        unit_names: List of unit names that belong to this component

    Returns:
        Tuple of (rst_file, list_of_output_files)
    """
    comp_info = comp_target[ComponentInfo]

    # Create RST file for this component
    comp_rst = ctx.actions.declare_file(ctx.label.name + "/components/" + comp_name + ".rst")

    # Collect requirements files - requirements depset contains File objects
    req_files = []
    req_refs = []
    if comp_info.requirements:
        doc_files = _filter_doc_files(comp_info.requirements.to_list())

        if doc_files:
            # Find common directory
            common_dir = _find_common_directory(doc_files)

            for f in doc_files:
                relative_path = _compute_relative_path(f, common_dir)
                output_file = _create_artifact_symlink(
                    ctx,
                    "components/" + comp_name + "_requirements",
                    f,
                    relative_path,
                )
                req_files.append(output_file)

                if _is_document_file(f):
                    doc_ref = ("components/" + comp_name + "_requirements/" + relative_path) \
                        .replace(".rst", "") \
                        .replace(".md", "")
                    req_refs.append("   " + doc_ref)

    # Collect test target names
    test_names = []
    if comp_info.tests:
        for test in comp_info.tests.to_list():
            test_names.append(test.label)

    # Generate RST content using template
    underline = "=" * (len("Component: " + comp_name))

    # Generate sections from template constants
    requirements_section = ""
    if req_refs:
        requirements_section = "\n" + _COMPONENT_REQUIREMENTS_SECTION_TEMPLATE.format(
            requirements_refs = "\n".join(req_refs),
        ) + "\n"

    units_section = ""
    if unit_names:
        unit_links = "\n".join(["- :doc:`../units/" + unit_name + "`" for unit_name in unit_names])
        units_section = "\n" + _COMPONENT_UNITS_SECTION_TEMPLATE.format(
            unit_links = unit_links,
        ) + "\n"

    tests_section = ""
    if test_names:
        test_list = "\n".join(["- ``" + str(test) + "``" for test in test_names])
        tests_section = "\n" + _TESTS_SECTION_TEMPLATE.format(
            entity_type = "component",
            test_list = test_list,
        ) + "\n"

    # Generate component RST content from template constant
    component_content = _COMPONENT_TEMPLATE.format(
        component_name = comp_name,
        underline = underline,
        requirements_section = requirements_section,
        units_section = units_section,
        implementation_section = "",
        tests_section = tests_section,
    )

    ctx.actions.write(
        output = comp_rst,
        content = component_content,
    )

    return (comp_rst, req_files)

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

    # Define artifacts
    # Note: "requirements" can contain both component_requirements and feature_requirements
    artifact_types = [
        "components",
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

    # Collect all units recursively from components
    all_units = _collect_units_recursive(ctx.attr.components)

    # Generate documentation for each unit
    unit_refs = []
    for unit_name, unit_target in all_units.items():
        unit_rst, unit_files = _generate_unit_doc(ctx, unit_target, unit_name)
        output_files.append(unit_rst)
        output_files.extend(unit_files)
        unit_refs.append("   units/" + unit_name)

    # Generate documentation for each component
    component_refs = []
    for comp_target in ctx.attr.components:
        if ComponentInfo in comp_target:
            comp_info = comp_target[ComponentInfo]
            comp_name = comp_info.name

            # Collect units that belong to this component
            comp_unit_names = []
            for nested in comp_info.components.to_list():
                if UnitInfo in nested:
                    comp_unit_names.append(nested.label.name)
                elif ComponentInfo in nested:
                    # For nested components, collect their units recursively
                    nested_units = _collect_units_recursive([nested])
                    comp_unit_names.extend(nested_units.keys())

            comp_rst, comp_files = _generate_component_doc(ctx, comp_target, comp_name, comp_unit_names)
            output_files.append(comp_rst)
            output_files.extend(comp_files)
            component_refs.append("   components/" + comp_name)

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
            "{units}": "\n".join(unit_refs) if unit_refs else "   (none)",
            "{components}": "\n".join(component_refs) if component_refs else "   (none)",
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
        "components": attr.label_list(
            default = [],
            doc = "Safety checklists targets or files.",
        ),
        "tests": attr.label_list(
            default = [],
            doc = "Integration tests for the dependable element.",
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
        <name>_index: Internal rule that generates index.rst and copies artifacts
        <name>: Main dependable element target (sphinx_module) with HTML documentation
        <name>_needs: Sphinx-needs JSON target (created by sphinx_module for cross-referencing)

    """

    # Step 1: Generate index.rst and collect all artifacts
    _dependable_element_index(
        name = name + "_index",
        module_name = name,
        description = description,
        template = Label("//bazel/rules/rules_score:templates/seooc_index.template.rst"),
        assumptions_of_use = assumptions_of_use,
        requirements = requirements,
        components = components,
        architectural_design = architectural_design,
        dependability_analysis = dependability_analysis,
        checklists = checklists,
        tests = tests,
        deps = deps,
        testonly = testonly,
        visibility = ["//visibility:private"],
    )

    # Step 2: Create sphinx_module using generated index and artifacts
    sphinx_module(
        name = name,
        srcs = [":" + name + "_index"],
        index = ":" + name + "_index",
        deps = deps,
        sphinx = sphinx,
        testonly = testonly,
        visibility = visibility,
    )
