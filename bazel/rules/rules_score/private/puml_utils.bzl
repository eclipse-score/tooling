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

"""Shared helper for generating RST wrapper pages and per-directory navigation
indexes for one architectural design view (static/dynamic/public_api/internal_api)."""

load("@bazel_skylib//lib:paths.bzl", "paths")

def relative_source_path(file, package, own_repo = None):
    """Return `file`'s path relative to `package`, or its full workspace-relative
    short_path when it doesn't live under `package` -- never just the bare
    basename, which would silently collide two same-named files that live in
    different directories outside the package.

    `file.short_path` carries a "../<repo>/" marker whenever `file` doesn't
    live in the build's *main* (root) repository -- even when it lives in the
    same repository as the rule consuming it, e.g. this architectural_design
    target's own package, when that target is itself built as someone else's
    dependency rather than as the build's root module. When `own_repo` (the
    consuming rule's own `ctx.label.workspace_name`) matches `file`'s owning
    label's repository, that marker is stripped first so staging paths are
    keyed off the rule's own repository, not whichever repository happens to
    be the build's root.
    """
    short_path = file.short_path
    owner = file.owner
    if own_repo and owner != None and owner.workspace_name == own_repo:
        own_repo_marker = "../" + own_repo + "/"
        if short_path.startswith(own_repo_marker):
            short_path = short_path[len(own_repo_marker):]
    prefix = package + "/" if package else ""
    if prefix and short_path.startswith(prefix):
        return short_path[len(prefix):]
    return short_path

def _directory_title(directory, root_title):
    """Human-readable title for a directory's index page.

    The root directory ("") uses the caller-supplied `root_title` (e.g. the
    view's display name, "Static Design"); nested directories are titled
    after their own last path segment.
    """
    if not directory:
        return root_title
    return directory.split("/")[-1].replace("_", " ").title()

def plan_view_layout(view_files, package, own_repo = None):
    """Plan how one architectural design view's files are staged, which
    diagram wrappers are generated, and which per-directory navigation
    indexes are generated -- reconciling generated navigation against any
    hand-authored ``.rst``/``.md`` pages so neither is silently dropped nor
    collides with the other on a staged path.

    Two coexistence rules:

    * A hand-authored ``index.rst``/``index.md`` in a directory is staged
      under a non-document name (``index.rst.inc``/``index.md.inc``) and
      ``.. include::``d into that directory's generated ``index.rst``.
      Navigation completeness is a correctness property (a missing entry is
      an orphaned page), so the generated index always owns that path; the
      author's title becomes the page's title and its own text renders
      above the generated toctree.
    * A hand-authored ``<stem>.rst``/``<stem>.md`` alongside a same-stem
      ``<stem>.puml``/``<stem>.plantuml`` suppresses that diagram's
      generated wrapper -- the wrapper is only a placeholder for prose that
      doesn't exist yet, so real authored content always wins. The ``.puml``
      is still staged as a sibling so the author's own ``.. uml::``
      resolves.

    A diagram literally named ``index`` is rejected via `errors`: that stem
    is reserved for the directory's own navigation page. A file from another
    repository -- one whose owning label's repository isn't `own_repo` --
    is also rejected via `errors`, since a symlinked staged path can't cross
    repository roots. So are two files that would occupy the same staged
    path. Rejected files are left out of `staged` entirely, so the returned
    plan stays free of collisions even though callers are expected to
    `fail()` on `errors`.

    Args:
        view_files: Iterable of File objects for one architectural design
                    view (e.g. ``ctx.files.static``).
        package:    ``ctx.label.package`` of the rule instantiating this view.
        own_repo:   ``ctx.label.workspace_name`` of the rule instantiating this
                    view -- see `relative_source_path`'s docstring for why this
                    (not the build's main repository) is the right reference
                    point for "does this file live in the same repository as
                    the target".

    Returns:
        Struct with:
          staged:  List of ``(File, staged_relative_path)`` to be symlinked
                   into the view's output tree, one per accepted input file.
          wrappers: List of ``(puml_file, relative_directory, stem)`` for
                   every diagram that still needs a generated RST wrapper.
          indexes: List of struct(directory, entries, body_relative_path)
                   describing one generated per-directory index.rst;
                   `entries` is a sorted list of toctree entry stems relative
                   to `directory`, `body_relative_path` is the staged path of
                   an authored index body to include, or None. Directories
                   that would only link on to a single child are skipped, so
                   the parent links straight to the first descendant with
                   content. Includes the root directory (``""``); empty
                   (``[]``) if the view has no navigable files at all.
          errors:  List of human-readable collision messages. Callers must
                   ``fail()`` on these rather than let a downstream
                   `declare_file()` collision surface as an opaque Bazel
                   "conflicting actions" error.
    """
    directories = {}
    stem_entries = {}
    entries_by_directory = {}
    index_bodies = {}
    staged = []
    staged_paths = {}
    errors = []

    def _register_directory(relative_directory):
        directories[relative_directory] = True
        parts = relative_directory.split("/") if relative_directory else []
        for part_count in range(1, len(parts) + 1):
            directories["/".join(parts[:part_count])] = True

    for f in view_files:
        relative_path = relative_source_path(f, package, own_repo)
        if relative_path.startswith("../"):
            errors.append(
                "'{}' lives outside this repository; architectural_design view files must live in the same repository as the target".format(f.short_path),
            )
            continue
        relative_directory = paths.dirname(relative_path)
        stem = paths.basename(relative_path)[:-(len(f.extension) + 1)] if f.extension else paths.basename(relative_path)

        if f.extension in ("rst", "md") and stem == "index":
            # Renamed so it's never picked up by Sphinx (source_suffix is
            # only .rst/.md) as a standalone document of its own.
            staged_path = relative_path + ".inc"
        else:
            staged_path = relative_path

        # Checked for every file, not just navigable ones: assets collide too,
        # and the ".inc" rename above can collide with a literally-named
        # "index.rst.inc" source.
        previous = staged_paths.get(staged_path)
        if previous != None:
            errors.append(
                "two files would both stage as '{}': '{}' and '{}'".format(staged_path, previous, f.short_path),
            )
            continue
        staged_paths[staged_path] = f.short_path
        staged.append((f, staged_path))

        if f.extension not in ("puml", "plantuml", "rst", "md"):
            continue

        _register_directory(relative_directory)
        stem_entries.setdefault((relative_directory, stem), {})[f.extension] = f

    wrappers = []
    for (relative_directory, stem), group in stem_entries.items():
        if "rst" in group and "md" in group:
            errors.append(
                "both '{stem}.rst' and '{stem}.md' exist for '{stem}' in directory '{dir}'; keep only one".format(
                    stem = stem,
                    dir = relative_directory or ".",
                ),
            )
            continue

        if "puml" in group and "plantuml" in group:
            errors.append(
                "both '{stem}.puml' and '{stem}.plantuml' exist for '{stem}' in directory '{dir}'; keep only one".format(
                    stem = stem,
                    dir = relative_directory or ".",
                ),
            )
            continue

        doc_ext = "rst" if "rst" in group else ("md" if "md" in group else None)
        doc_file = group.get(doc_ext) if doc_ext else None
        puml_file = group.get("puml") or group.get("plantuml")

        if stem == "index":
            if puml_file:
                errors.append(
                    "'{}' is named 'index', which is reserved for the generated directory navigation page; rename it".format(puml_file.short_path),
                )
                continue
            if doc_file:
                index_bodies[relative_directory] = relative_source_path(doc_file, package, own_repo) + ".inc"
            continue

        if puml_file and not doc_file:
            wrappers.append((puml_file, relative_directory, stem))
        entries_by_directory.setdefault(relative_directory, []).append(stem)

    if not directories:
        return struct(staged = staged, wrappers = [], indexes = [], errors = errors)

    # Seeded only once a navigable file exists, so an empty view generates
    # no index at all.
    directories[""] = True

    def _children_of(directory):
        directory_prefix = directory + "/" if directory else ""
        return [
            child
            for child in sorted(directories.keys())
            if child != directory and child.startswith(directory_prefix) and "/" not in child[len(directory_prefix):]
        ]

    # A directory with no pages, no authored body and exactly one child
    # contributes a navigation page whose only link is the next one down.
    # Drop it and let its parent link straight through to its only
    # descendant that has something to show. The root is always kept: it is
    # the view's single entry point.
    collapsed = {}
    for directory in directories.keys():
        if not directory or index_bodies.get(directory) or entries_by_directory.get(directory):
            continue
        if len(_children_of(directory)) == 1:
            collapsed[directory] = True

    def _resolve_entry(directory):
        for _ in range(len(directories)):
            if not collapsed.get(directory):
                break
            directory = _children_of(directory)[0]
        return directory

    indexes = []
    for directory in sorted(directories.keys()):
        if collapsed.get(directory):
            continue
        entries = list(entries_by_directory.get(directory, []))
        directory_prefix = directory + "/" if directory else ""
        for child in _children_of(directory):
            resolved = _resolve_entry(child)
            entries.append(resolved[len(directory_prefix):] + "/index")

        indexes.append(struct(
            directory = directory,
            entries = sorted(entries),
            body_relative_path = index_bodies.get(directory),
        ))

    return struct(staged = staged, wrappers = wrappers, indexes = indexes, errors = errors)

def emit_view_navigation(ctx, plan, output_dir, template, root_title, colocated_by_relative_path):
    """Declare the wrapper and per-directory index files described by a
    `plan_view_layout()` plan.

    Args:
        ctx:        Rule context.
        plan:       Struct returned by `plan_view_layout()`.
        output_dir: String prefix for declared output files
                    (e.g. ``ctx.label.name``).
        template:   The ``puml_diagram.template.rst`` File (from
                    ``ctx.file._puml_rst_template``).
        root_title: Title for the view's top-level index page (e.g.
                    ``"Static Design"``), used when that page has no
                    authored body of its own.
        colocated_by_relative_path: Dict from `plan.staged` relative path to
                    its colocated File (see `_colocate_view_files`) -- needed
                    to resolve a single hand-authored page (not a generated
                    wrapper) as the root index, since that page's real
                    on-disk location is the colocated copy, not the
                    original source File.

    Returns:
        Struct with:
          wrappers:   List of declared ``.rst`` wrapper Files, excluding
                      `root_index` if it turned out to be a wrapper.
          indexes:    List of declared per-directory ``index.rst`` Files,
                      excluding the root index.
          root_index: The view's single top-level toctree entry: the
                      generated ``index.rst``, or -- when the whole view is
                      just one file with no authored index body, making that
                      index a redundant pass-through -- that one page
                      directly. None if `plan` has no indexes at all.
    """
    wrappers = []
    wrapper_by_stem = {}
    for puml_file, relative_directory, stem in plan.wrappers:
        title = stem.replace("_", " ").title()
        wrapper_relative_path = paths.join(relative_directory, stem + ".rst")
        wrapper = ctx.actions.declare_file(
            "{}/{}".format(output_dir, wrapper_relative_path),
        )
        ctx.actions.expand_template(
            template = template,
            output = wrapper,
            substitutions = {
                "{title}": title,
                "{underline}": "=" * len(title),
                "{basename}": puml_file.basename,
            },
        )
        wrappers.append(wrapper)
        if relative_directory == "":
            wrapper_by_stem[stem] = wrapper

    if not plan.indexes:
        return struct(wrappers = wrappers, indexes = [], root_index = None)

    # A view with exactly one navigable file and no authored index body has
    # nothing to navigate: a generated root index would be a "<root_title>"
    # page linking only to that one page, repeating the same title three
    # times in the sidebar alongside the caller's own section heading for
    # this view. Surface that one page as the root index instead.
    root_plan = plan.indexes[0]
    if len(plan.indexes) == 1 and not root_plan.body_relative_path and len(root_plan.entries) == 1:
        stem = root_plan.entries[0]
        sole_page = (
            wrapper_by_stem.get(stem) or
            colocated_by_relative_path.get(stem + ".rst") or
            colocated_by_relative_path.get(stem + ".md")
        )
        if sole_page != None:
            return struct(
                wrappers = [w for w in wrappers if w != sole_page],
                indexes = [],
                root_index = sole_page,
            )

    root_index = None
    indexes = []
    for index_plan in plan.indexes:
        if index_plan.body_relative_path:
            # Compose with the authored body instead of emitting a title --
            # see plan_view_layout's docstring.
            body_basename = paths.basename(index_plan.body_relative_path)
            parser_option = "\n   :parser: myst_parser.sphinx_" if body_basename.endswith(".md.inc") else ""
            preamble = ".. include:: {}{}\n\n".format(body_basename, parser_option)
        else:
            title = _directory_title(index_plan.directory, root_title)
            preamble = ":score-directory-index:\n\n{}\n{}\n\n".format(title, "-" * len(title))

        index = ctx.actions.declare_file(
            "{}/{}".format(output_dir, paths.join(index_plan.directory, "index.rst")),
        )
        ctx.actions.write(
            output = index,
            content = "{}.. toctree::\n   :maxdepth: 1\n\n{}\n".format(
                preamble,
                "\n".join(["   " + entry for entry in index_plan.entries]),
            ),
        )
        if index_plan.directory == "":
            root_index = index
        else:
            indexes.append(index)

    return struct(wrappers = wrappers, indexes = indexes, root_index = root_index)
