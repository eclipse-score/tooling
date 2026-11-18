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

"""Repository-name-independent identities for manual analysis lock entries."""

def normalize_repo_name(repo):
    """Strips the canonical-name decoration Bazel adds to module repositories.

    `seooc+`, `seooc~` and `seooc~1.0` become `seooc`; the extension repository
    `rules_foo++ext+repo` (or `rules_foo~~ext~repo`) becomes `rules_foo+ext+repo`.
    A root-module extension repository (`+ext+repo`) is kept unchanged.
    """
    parts = repo.replace("~", "+").split("+")
    if len(parts) == 1 or not parts[0]:
        return repo
    return "+".join([parts[0]] + parts[2:])

def lock_file_id(file, repository):
    """Returns the lock identity of an input file.

    Files of the analysis repository are repository-relative; other external
    files keep `../<repo>/` with a normalized repo name.
    """
    path = file.short_path
    if not path.startswith("../"):
        return path
    repo, _, relative_path = path[3:].partition("/")
    if file.owner != None and file.owner.repo_name == repository:
        return relative_path
    return "../{}/{}".format(normalize_repo_name(repo), relative_path)

def lock_rule_entry(entry, repository_prefix):
    """Returns a `<label>\\t<canonical_form>` entry with a normalized repo name."""
    label, canonical_form = entry.split("\t", 1)
    prefix, relative_label = label.split("//", 1)
    if prefix == repository_prefix:
        label = "@@//" + relative_label
    elif prefix:
        label = "@{}//{}".format(normalize_repo_name(prefix.lstrip("@")), relative_label)
    return "{}\t{}".format(label, canonical_form)
