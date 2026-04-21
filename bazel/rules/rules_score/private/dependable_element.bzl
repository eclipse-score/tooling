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
    "@lobster//:lobster.bzl",
    "subrule_lobster_html_report",
    "subrule_lobster_report",
)
load(
    "//bazel/rules/rules_score:providers.bzl",
    "ArchitecturalDesignInfo",
    "CertifiedScope",
    "ComponentInfo",
    "DependabilityAnalysisInfo",
    "DependableElementInfo",
    "DependableElementLobsterInfo",
    "FeatureRequirementsInfo",
    "SphinxModuleInfo",
    "SphinxNeedsInfo",
    "SphinxSourcesInfo",
    "UnitInfo",
)
load(
    "//bazel/rules/rules_score/private:architecture_aspect.bzl",
    "CurrentArchitectureProviderInfo",
    "collect_current_architecture_aspect",
)
load(
    "//bazel/rules/rules_score/private:lobster_config.bzl",
    "format_lobster_sources",
)
load("//bazel/rules/rules_score/private:sphinx_module.bzl", "sphinx_module")

# ============================================================================
# Template Constants
# ============================================================================

_UNIT_DESIGN_SECTION_TEMPLATE = """Unit Design
-----------

{design_includes}"""

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

.. toctree::
   :maxdepth: 3

{unit_toctree_refs}"""

_UNIT_TEMPLATE = """

{unit_name}
{underline}

{design_section}{implementation_section}{tests_section}"""

_COMPONENT_TEMPLATE = """

{component_name}
{underline}

{requirements_section}{units_section}{implementation_section}{tests_section}"""

# ============================================================================
# Integrity Level Definitions
# ============================================================================

# Allowed integrity levels in ascending order of stringency (A = lowest, D = highest).
_INTEGRITY_LEVELS = ["QM", "A", "B", "C", "D"]

# Maps each integrity level to a numeric rank for comparison.
_INTEGRITY_LEVEL_RANK = {level: rank for rank, level in enumerate(_INTEGRITY_LEVELS)}

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
    return [f for f in files if f.extension in ["rst", "md", "puml", "plantuml", "png", "svg", "inc", "json"]]

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
    design_include_lines = []
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
                    # Inline the design RST directly into the unit page.
                    # Path is relative to the unit RST (which lives in units/).
                    include_path = unit_name + "_design/" + relative_path
                    design_include_lines.append(".. include:: " + include_path)
                    design_include_lines.append("")
                elif f.extension in ("puml", "plantuml"):
                    # sphinxcontrib-plantuml resolves .. uml:: paths relative to
                    # the *including* document (units/unit_name.rst), not the
                    # included RST file. Symlink the PUML alongside the unit RST
                    # so the directive inside the included RST can find it.
                    # NOTE: All PUML files across all units must have unique
                    # basenames because they share the flat units/ namespace.
                    # Use unit-specific filenames (e.g. unit_1_class_diagram.puml)
                    # to avoid declaration conflicts.
                    sibling = _create_artifact_symlink(ctx, "units", f, f.basename)
                    design_files.append(sibling)

    # Collect implementation target names
    impl_names = []
    if unit_info.implementation:
        for impl in unit_info.implementation.to_list():
            impl_names.append(impl.label)

    # Collect test target names
    test_names = []
    if unit_info.tests:
        for test in unit_info.tests.to_list():
            test_names.append(test.short_path)

    # Generate RST content using template
    underline = "=" * len(unit_name)

    # Generate sections from template constants
    design_section = ""
    if design_include_lines:
        design_section = "\n" + _UNIT_DESIGN_SECTION_TEMPLATE.format(
            design_includes = "\n".join(design_include_lines),
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

    # Collect requirements files from SphinxSourcesInfo (ComponentInfo.requirements holds
    # only lobster traceability files; Sphinx docs are carried via SphinxSourcesInfo)
    req_files = []
    req_refs = []
    if SphinxSourcesInfo in comp_target:
        doc_files = _filter_doc_files(comp_target[SphinxSourcesInfo].srcs.to_list())

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
                    doc_ref = (comp_name + "_requirements/" + relative_path) \
                        .replace(".rst", "") \
                        .replace(".md", "")
                    req_refs.append("   " + doc_ref)

    # Generate RST content using template
    underline = "=" * len(comp_name)

    # Generate sections from template constants
    requirements_section = ""
    if req_refs:
        requirements_section = "\n" + _COMPONENT_REQUIREMENTS_SECTION_TEMPLATE.format(
            requirements_refs = "\n".join(req_refs),
        ) + "\n"

    units_section = ""
    if unit_names:
        unit_toctree_refs = "\n".join(["   ../units/" + name for name in unit_names])
        units_section = "\n" + _COMPONENT_UNITS_SECTION_TEMPLATE.format(
            unit_toctree_refs = unit_toctree_refs,
        ) + "\n"

    tests_section = ""

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
# Architecture Verification Helper Function
# ============================================================================

def _collect_architecture_components(ctx):
    """Collect all architecture component entries for the dependable element.

    Gathers component/unit info from the collect_current_architecture_aspect on components and
    adds a top-level entry for the dependable element itself.

    Args:
        ctx: Rule context

    Returns:
        Dict mapping component/unit names to their architecture entries
    """

    # Collect architecture info from aspect on components
    all_components = {}
    for comp in ctx.attr.components:
        if CurrentArchitectureProviderInfo in comp:
            info = comp[CurrentArchitectureProviderInfo]
            all_components.update(info.components)

    # Build the top-level entry for the dependable element itself
    de_components = []
    de_units = []
    for comp in ctx.attr.components:
        if ComponentInfo in comp:
            de_components.append(str(comp.label))
        elif UnitInfo in comp:
            de_units.append(str(comp.label))

    de_entry = {}
    if de_components:
        de_entry["components"] = de_components
    if de_units:
        de_entry["units"] = de_units

    # Guard against silently overwriting a key already collected by the aspect
    if ctx.attr.module_name in all_components:
        fail("module_name '{}' conflicts with an existing component key collected by the architecture aspect".format(ctx.attr.module_name))
    all_components[ctx.attr.module_name] = de_entry

    return all_components

def _run_archver_validation(ctx, arch_json, static_fbs_files):
    """Run the architecture verifier tool against a pre-built JSON file.

    Args:
        ctx: Rule context
        arch_json: The architecture JSON File object (already declared and written)
        static_fbs_files: List of static FlatBuffer files to verify against

    Returns:
        archver_log File object
    """

    archver_log = ctx.actions.declare_file(ctx.label.name + "/archver.log")

    archver_args = ctx.actions.args()
    archver_args.add("--architecture-json", arch_json)
    archver_args.add_all("--static-fbs", static_fbs_files)
    archver_args.add("--output", archver_log)

    # ctx.actions.run will fail the build if archver returns non-zero exit code
    ctx.actions.run(
        inputs = [arch_json] + static_fbs_files,
        outputs = [archver_log],
        executable = ctx.executable._archver,
        arguments = [archver_args],
        progress_message = "Verifying architecture: %s" % ctx.label.name,
        mnemonic = "ArchVerify",
    )

    return archver_log

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

    # =========================================================================
    # Sphinx Documentation: process artifact files and generate RST pages
    # =========================================================================

    # Declare output index file
    index_rst = ctx.actions.declare_file(ctx.label.name + "/index.rst")
    output_files = [index_rst]

    # Process each well-known artifact type into symlinked output files and
    # toctree references for the index template.
    # "requirements" covers both feature_requirements and component_requirements.
    artifact_types = [
        "assumptions_of_use",
        "requirements",
        "architectural_design",
        "dependability_analysis",
        "checklists",
    ]

    artifacts_by_type = {}
    for artifact_name in artifact_types:
        files, refs = _process_artifact_type(ctx, artifact_name)
        output_files.extend(files)
        artifacts_by_type[artifact_name] = refs

    # Collect all units recursively from components
    all_units = _collect_units_recursive(ctx.attr.components)

    # Generate a dedicated RST page for each unit. Unit pages are referenced
    # via toctree from the component RST page (see _generate_component_doc).
    for unit_name, unit_target in all_units.items():
        unit_rst, unit_files = _generate_unit_doc(ctx, unit_target, unit_name)
        output_files.append(unit_rst)
        output_files.extend(unit_files)

    # Generate a dedicated RST page for each component and collect certification
    # metadata (certified scopes, dependent labels) for later validation.
    component_refs = []
    collected_certified_scopes = []
    collected_dependent_labels = []
    for comp_target in ctx.attr.components:
        if ComponentInfo in comp_target:
            comp_info = comp_target[ComponentInfo]
            comp_name = comp_info.name

            # Collect direct unit names for this component's RST page
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
            component_refs.append(comp_name)

            if comp_info.dependent_labels:
                collected_dependent_labels.append(comp_info.dependent_labels)
        if CertifiedScope in comp_target:
            collected_certified_scopes.append(comp_target[CertifiedScope].transitive_scopes)

    # Collect CertifiedScope from processed_deps as well (other dependable elements)
    for dep in ctx.attr.processed_deps:
        if CertifiedScope in dep:
            collected_certified_scopes.append(dep[CertifiedScope].transitive_scopes)

    # Generate an intermediate components/index.rst that groups all component pages
    # under a single "Components" navigation entry in the Sphinx sidebar.
    if component_refs:
        comp_index_rst = ctx.actions.declare_file(ctx.label.name + "/components/index.rst")
        comp_index_underline = "=" * len("Components")
        comp_toctree_entries = "\n   ".join(component_refs)
        ctx.actions.write(
            output = comp_index_rst,
            content = "Components\n" + comp_index_underline + "\n\n.. toctree::\n   :maxdepth: 2\n\n   " + comp_toctree_entries + "\n",
        )
        output_files.append(comp_index_rst)
        components_ref = "components/index"
    else:
        components_ref = ""

    # Generate submodule links for the index page
    deps_links = _process_deps(ctx)

    # Render the index.rst using the template
    title = ctx.attr.module_name
    underline = "=" * len(title)

    ctx.actions.expand_template(
        template = ctx.file.template,
        output = index_rst,
        substitutions = {
            "{title}": title,
            "{underline}": underline,
            "{description}": ctx.attr.description,
            "{components}": components_ref,
            "{assumptions_of_use}": "\n   ".join(artifacts_by_type["assumptions_of_use"]),
            "{requirements}": "\n   ".join(artifacts_by_type["requirements"]),
            "{architectural_design}": "\n   ".join(artifacts_by_type["architectural_design"]),
            "{dependability_analysis}": "\n   ".join(artifacts_by_type["dependability_analysis"]),
            "{checklists}": "\n   ".join(artifacts_by_type["checklists"]),
            "{submodules}": deps_links,
        },
    )

    # =========================================================================
    # Architecture Verification: build current-architecture JSON and run archver
    # =========================================================================

    # Collect the current architecture from all components (via aspect) and
    # write it as JSON consumed by the architecture verifier tool.
    all_components = _collect_architecture_components(ctx)

    arch_json = ctx.actions.declare_file(ctx.label.name + "/architecture.json")
    ctx.actions.write(
        output = arch_json,
        content = json.encode_indent({"components": all_components}, indent = "  "),
    )

    # Collect static FlatBuffers from architectural_design targets (the expected
    # static architecture) and verify them against the current architecture.
    static_fbs_files = []
    for ad in ctx.attr.architectural_design:
        if ArchitecturalDesignInfo in ad:
            static_fbs_files.extend(ad[ArchitecturalDesignInfo].static.to_list())

    # Run architecture verifier; build fails automatically on non-zero exit
    archver_log = _run_archver_validation(ctx, arch_json, static_fbs_files)

    # Both outputs are included so archver always runs in a default build.
    # archver_log is also exposed in the debug output group for explicit access.
    output_files.append(arch_json)
    output_files.append(archver_log)

    # =========================================================================
    # Safety Certification Validation: certified scope and integrity level checks
    # =========================================================================

    # Verify that all transitive implementation dependencies are within the
    # certified scope declared for this dependable element.
    # @todo: Make this check aware of the repository (@foo) for example.
    certified_scopes = depset(transitive = collected_certified_scopes).to_list()
    tree = {}
    for certified_scope in certified_scopes:
        certified_scope = Label(certified_scope)
        node = tree
        path = certified_scope.package.split("/")
        last_element = path.pop()
        for path in path:
            node = node.setdefault(path, default = {})

        if type(node) == type([]):
            node.append(certified_scope.name)
        else:
            inserted_element = node.setdefault(last_element, default = [certified_scope.name])
            if inserted_element != [certified_scope.name]:
                fail("The same scope is covered twice: {}".format(certified_scope))

    dependencies = depset(transitive = collected_dependent_labels).to_list()
    for dep in dependencies:
        node = tree
        for path in dep.package.split("/"):
            if type(node) == type([]):
                if "__subpackages__" in node:
                    break
                elif dep.name in node:
                    break
                else:
                    fail("Not in certified scope {}, stopping at {}".format(dep, path))

            child = node.get(path)
            if child == None:
                fail("Not in certified scope {}, stopping at {}".format(dep, path))
            else:
                node = child

    # Integrity-level check: a dependable element must not depend on elements
    # with a lower integrity level than its own (D > C > B > A).
    own_rank = _INTEGRITY_LEVEL_RANK[ctx.attr.integrity_level]
    for dep in ctx.attr.processed_deps:
        if DependableElementInfo in dep:
            dep_info = dep[DependableElementInfo]
            dep_rank = _INTEGRITY_LEVEL_RANK[dep_info.integrity_level]
            if dep_rank < own_rank:
                fail(
                    "Integrity level violation: '{}' (level {}) depends on '{}' (level {}). " +
                    "A dependable element must not depend on elements with a lower integrity level.",
                    ctx.label,
                    ctx.attr.integrity_level,
                    dep_info.label,
                    dep_info.integrity_level,
                )

    # =========================================================================
    # Lobster Traceability: Dependable Element Level
    # Builds a three-tier report: Feature Requirements <- Component Requirements
    # <- Unit Tests (gtest). The report is only produced when all three tiers
    # are present; gtest is optional (component-only traceability is possible).
    # =========================================================================

    # Collect feature requirement .lobster files from requirements targets
    feat_req_lobster_files = []
    for req_target in ctx.attr.requirements:
        if FeatureRequirementsInfo in req_target:
            feat_req_lobster_files.append(req_target[FeatureRequirementsInfo].srcs)

    feat_req_lobster_depset = depset(transitive = feat_req_lobster_files)

    # Collect component requirement and test .lobster files from ComponentInfo
    comp_req_lobster_files = []
    comp_test_lobster_files = []
    comp_arch_lobster_files = []
    for comp_target in ctx.attr.components:
        if ComponentInfo in comp_target:
            comp_info = comp_target[ComponentInfo]
            if comp_info.requirements:
                comp_req_lobster_files.append(comp_info.requirements)
            if comp_info.tests:
                comp_test_lobster_files.append(comp_info.tests)
            if comp_info.architecture:
                comp_arch_lobster_files.append(comp_info.architecture)

    comp_req_lobster_depset = depset(transitive = comp_req_lobster_files)
    comp_test_lobster_depset = depset(transitive = comp_test_lobster_files)
    comp_arch_lobster_depset = depset(transitive = comp_arch_lobster_files)

    # Collect safety analysis lobster files from dependability_analysis targets
    sa_lobster_files = {}  # canonical name -> File, merged from all DA targets
    for da_target in ctx.attr.dependability_analysis:
        if DependabilityAnalysisInfo in da_target:
            da_info = da_target[DependabilityAnalysisInfo]
            sa_lobster_files.update(da_info.lobster_files)

    # Collect public api lobster files from architectural_design targets
    public_api_lobster_list = []
    for ad_target in ctx.attr.architectural_design:
        if ArchitecturalDesignInfo in ad_target:
            public_api_lobster_list += ad_target[ArchitecturalDesignInfo].public_api_lobster_files.to_list()

    # Build the DE-level lobster report if feature and component traces exist
    feat_req_list = feat_req_lobster_depset.to_list()
    comp_req_list = comp_req_lobster_depset.to_list()
    comp_test_list = comp_test_lobster_depset.to_list()
    comp_arch_list = comp_arch_lobster_depset.to_list()
    interface_req_list = public_api_lobster_list
    fm_list = [sa_lobster_files["failuremodes.lobster"]] if "failuremodes.lobster" in sa_lobster_files else []
    cm_list = [sa_lobster_files["controlmeasures.lobster"]] if "controlmeasures.lobster" in sa_lobster_files else []
    rc_list = [sa_lobster_files["root_causes.lobster"]] if "root_causes.lobster" in sa_lobster_files else []

    lobster_report_file = None
    lobster_html_report = None
    lobster_files = []
    if feat_req_list and comp_req_list:
        lobster_config = ctx.actions.declare_file(ctx.label.name + "/de_traceability_config")
        ctx.actions.expand_template(
            template = ctx.file._lobster_de_template,
            output = lobster_config,
            substitutions = {
                "{FEAT_REQ_SOURCES}": format_lobster_sources(feat_req_list),
                "{COMP_REQ_SOURCES}": format_lobster_sources(comp_req_list),
                "{ARCH_SOURCES}": format_lobster_sources(comp_arch_list),
                "{UNIT_TEST_SOURCES}": format_lobster_sources(comp_test_list),
                "{PUBLIC_API_SOURCES}": format_lobster_sources(interface_req_list),
                "{FM_SOURCES}": format_lobster_sources(fm_list),
                "{CM_SOURCES}": format_lobster_sources(cm_list),
                "{RC_SOURCES}": format_lobster_sources(rc_list),
            },
        )

        all_lobster_inputs = feat_req_list + comp_req_list + comp_arch_list + comp_test_list + interface_req_list + fm_list + cm_list + rc_list
        lobster_report_file = subrule_lobster_report(all_lobster_inputs, lobster_config)
        lobster_html_report = subrule_lobster_html_report(lobster_report_file)

        lobster_files = [lobster_config, lobster_report_file, lobster_html_report]
        output_files.extend(lobster_files)

    return [
        DefaultInfo(files = depset(output_files)),
        CertifiedScope(transitive_scopes = depset(transitive = collected_certified_scopes)),
        DependableElementInfo(
            integrity_level = ctx.attr.integrity_level,
            label = ctx.label,
        ),
        DependableElementLobsterInfo(
            lobster_report = lobster_report_file,
            lobster_html_report = lobster_html_report,
        ),
        OutputGroupInfo(debug = depset([archver_log])),
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
            providers = [FeatureRequirementsInfo],
            doc = "Feature requirements targets (feature_requirements only).",
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
            mandatory = True,
            aspects = [collect_current_architecture_aspect],
            doc = "Component targets (aspect is applied here and passed to subrule).",
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
        "processed_deps": attr.label_list(
            default = [],
            doc = "Dependencies on other dependable element modules (submodules).",
        ),
        "integrity_level": attr.string(
            mandatory = True,
            values = _INTEGRITY_LEVELS,
            doc = "Integrity level of the dependable element. Allowed values: 'A', 'B', 'C', 'D' (D > C > B > A).",
        ),
        "_archver": attr.label(
            default = Label("//validation/archver"),
            executable = True,
            cfg = "exec",
            doc = "Architecture verifier tool",
        ),
        "_lobster_de_template": attr.label(
            default = Label("//bazel/rules/rules_score/lobster/config:lobster_de_template"),
            allow_single_file = True,
            doc = "Lobster config template for dependable element traceability.",
        ),
    },
    subrules = [subrule_lobster_report, subrule_lobster_html_report],
)

# ============================================================================
# Main Dependable Element Rule (test)
# ============================================================================

def _dependable_element_impl(ctx):
    """Implementation for the main dependable_element test rule.

    * Forwards SphinxModuleInfo / SphinxNeedsInfo so other modules can use
      this target directly as a Sphinx dependency.
    * Forwards CertifiedScope and DependableElementInfo for safety checks.
    * Builds a test executable that runs lobster-ci-report on the
      pre-built lobster JSON report, so ``bazel test <name>`` validates
      traceability.
    """
    sphinx_dep = ctx.attr.sphinx_module_dep
    index_dep = ctx.attr.index_dep

    # --- Lobster traceability test executable --------------------------------
    lobster_info = index_dep[DependableElementLobsterInfo]
    test_executable = ctx.actions.declare_file(ctx.label.name + "_lobster_test")

    if lobster_info.lobster_report != None:
        command = "set -o pipefail; {ci} {report}".format(
            ci = ctx.executable._lobster_ci_report.short_path,
            report = lobster_info.lobster_report.short_path,
        )
        runfiles = ctx.runfiles(
            files = [ctx.executable._lobster_ci_report, lobster_info.lobster_report],
        ).merge(ctx.attr._lobster_ci_report[DefaultInfo].default_runfiles)
    else:
        command = "exit 0"
        runfiles = ctx.runfiles()

    ctx.actions.write(
        output = test_executable,
        content = command,
        is_executable = True,
    )

    # Compose default outputs: the two lobster report files (JSON + HTML) and
    # the Sphinx HTML documentation. Intermediate files from the index (RST
    # sources, lobster config, etc.) are intentionally excluded.
    lobster_default_files = []
    if lobster_info.lobster_report:
        lobster_default_files.append(lobster_info.lobster_report)
    if lobster_info.lobster_html_report:
        lobster_default_files.append(lobster_info.lobster_html_report)

    return [
        # DefaultInfo: two lobster report files + Sphinx HTML docs so that
        # ``bazel build <name>`` produces exactly the final user-facing outputs.
        DefaultInfo(
            executable = test_executable,
            files = depset(lobster_default_files, transitive = [sphinx_dep[DefaultInfo].files]),
            runfiles = runfiles,
        ),
        # Sphinx docs providers: forwarded from sphinx_module so callers can use
        # <name> directly as a Sphinx dependency
        sphinx_dep[SphinxModuleInfo],
        # Safety providers: forwarded from index for certification scope and
        # integrity-level checks by parent dependable elements
        index_dep[CertifiedScope],
        index_dep[DependableElementInfo],
    ] + ([sphinx_dep[SphinxNeedsInfo]] if SphinxNeedsInfo in sphinx_dep else [])

_dependable_element_test = rule(
    implementation = _dependable_element_impl,
    doc = """Main dependable element target.

    Wraps the sphinx_module and exposes a lobster traceability test.
    Running ``bazel test <name>`` executes lobster-ci-report to verify
    that all traceability links are satisfied.
    """,
    attrs = {
        "index_dep": attr.label(
            mandatory = True,
            doc = "The <name>_index target generated by _dependable_element_index.",
        ),
        "sphinx_module_dep": attr.label(
            mandatory = True,
            doc = "The <name>_doc sphinx_module target providing SphinxModuleInfo.",
        ),
        "_lobster_ci_report": attr.label(
            default = Label("@lobster//:lobster-ci-report"),
            executable = True,
            cfg = "exec",
            doc = "Lobster CI report tool used to validate traceability at test time.",
        ),
    },
    test = True,
)

# ============================================================================
# Public Macro
# ============================================================================
# lobster-trace: Tools.ArchitectureModelingDependableElement
def dependable_element(
        name,
        description,
        assumptions_of_use,
        requirements,
        architectural_design,
        dependability_analysis,
        components,
        tests,
        integrity_level,
        checklists = [],
        deps = [],
        sphinx = Label("//bazel/rules/rules_score:score_build"),
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
        integrity_level: Integrity level of the dependable element. Allowed values:
            'A', 'B', 'C', 'D' (D > C > B > A). A dependable element must not
            depend on elements with a lower integrity level than its own.
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

    processed_deps = []
    for dep in deps:
        processed_deps.append("{}_index".format(dep))

    # Step 1: Generate index.rst and collect all artifacts
    # Note: archver validation runs as a subrule within the index generation
    _dependable_element_index(
        name = name + "_index",
        module_name = name,
        description = description,
        template = Label("//bazel/rules/rules_score:templates/dependable_element_index.template.rst"),
        assumptions_of_use = assumptions_of_use,
        requirements = requirements,
        components = components,
        architectural_design = architectural_design,
        dependability_analysis = dependability_analysis,
        checklists = checklists,
        tests = tests,
        deps = deps,
        processed_deps = processed_deps,
        integrity_level = integrity_level,
        testonly = testonly,
        visibility = visibility,
    )

    # Step 2: Create sphinx_module using generated index and artifacts.
    # Internal deps use the <dep>_doc variant so that sphinx_module can resolve
    # <dep>_doc (SphinxModuleInfo) and <dep>_doc_needs (SphinxNeedsInfo).
    sphinx_module(
        name = name + "_doc",
        srcs = [":" + name + "_index"],
        index = ":" + name + "_index",
        deps = [d + "_doc" for d in deps],
        sphinx = sphinx,
        testonly = testonly,
        visibility = visibility,
    )

    # Step 3: Create the main <name> target:
    # - is a test rule (bazel test <name> runs the lobster check)
    # - forwards SphinxModuleInfo / SphinxNeedsInfo so callers can use it as a
    #   sphinx dependency without knowing about the internal _doc split
    _dependable_element_test(
        name = name,
        index_dep = ":" + name + "_index",
        sphinx_module_dep = ":" + name + "_doc",
        visibility = visibility,
    )
