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
"""Tests for the core orchestration functions in sbom_generator.py.

What this file tests
---------------------
filter_repos()
  - Repos matching an exclude_pattern are removed.
  - crate_index__ / crates_io__ / _crates__ repos are always kept regardless
    of patterns — they are real dependencies, not build tools.
  - Multiple patterns are each applied independently.

resolve_component() — all repo-type branches
  - bazel_dep module  : version, PURL, sha256 → checksum, pedigree fields.
  - bzlmod "+" suffix : repo name "boost+" resolves to component name "boost".
  - http_archive      : version, URL, license, sha256 → checksum; absent sha256
                        means no checksum key on the result.
  - git_repository    : URL, license; commit_date replaces version when
                        version == "unknown".
  - Crate from cache  : direct lookup, hyphen/underscore normalisation, checksum.
  - crate_universe bzlmod format (rules_rust++crate+crate_index__NAME-VER):
      simple crate name, complex platform-suffix name
      (iceoryx2-bb-lock-free-qnx8-0.7.0), metadata enrichment, no-cache fallback.
  - Legacy crates_io__NAME-VERSION format + metadata enrichment.
  - score_-prefixed repos → Eclipse Foundation supplier.
  - Dot sub-library (boost.config+) inherits version/license/checksum from the
    parent entry (works for modules, http_archives, and git_repositories).
  - Unknown repos → placeholder dict with version = "unknown"; never None.
  - Return type is always dict, never None, for any input.

deduplicate_components()
  - No duplicates → list unchanged.
  - Exact duplicate → keep first entry.
  - Known version preferred over "unknown".
  - Entry with license preferred over entry without license.
  - Empty input returns empty list.

parse_module_bazel_files()
  - Extracts name, version, PURL from a module() call.
  - Missing or unreadable files are silently skipped.
  - Files without a module() block are skipped.
  - Multiple files are merged into one dict.
  - Multiline module() blocks with extra attributes are handled.

parse_module_lockfiles()
  - Extracts version from registryFileHashes MODULE.bazel URL keys.
  - Extracts sha256 from source.json URL keys.
  - Modules with conflicting (ambiguous) versions are excluded.
  - Missing, malformed JSON, or empty files are silently skipped.
  - Multiple lockfiles are merged.
  - version appears inside the purl string.

mark_missing_cpp_descriptions()
  - "Missing" is injected for non-Rust library components with no description.
  - pkg:cargo/ crates are never marked "Missing".
  - Components with an existing description are not modified.
  - Non-library types (application, etc.) are not marked.
  - Mixed component lists are handled independently per component.

main() — end-to-end integration
  - Returns 0 on success.
  - Writes a valid SPDX 2.3 JSON file when --spdx-output is given.
  - Writes a valid CycloneDX 1.6 JSON file when --cyclonedx-output is given.
  - A component present in metadata appears in both output files.
  - The declared component_name is excluded from its own dependency list.
  - BCR known licenses (e.g. boost.config → BSL-1.0) are applied before output.
  - crate_universe repos resolve and appear in output.
  - Exclude patterns remove repos from output.
  - component_version is auto-detected from metadata["modules"] when not in config.
  - dep_module_files: MODULE.bazel version flows into output.
  - module_lockfiles: lockfile-derived version flows into output.
  - --crates-cache: external crate metadata cache enriches crate components.
  - --cdxgen-sbom: C++ enrichment data applied to matching components.
  - Requesting only --spdx-output does not create a CycloneDX file.
  - Requesting only --cyclonedx-output does not create an SPDX file.

Bazel target : //sbom/tests:test_sbom_generator
Run          : bazel test //sbom/tests:test_sbom_generator
               PYTHONPATH=. pytest sbom/tests/test_sbom_generator.py -v
"""

import json
import os
import shutil
import tempfile
import unittest
import unittest.mock

from sbom.internal.generator.sbom_generator import (
    deduplicate_components,
    filter_repos,
    main,
    mark_missing_cpp_descriptions,
    parse_module_bazel_files,
    parse_module_lockfiles,
    resolve_component,
)


# ---------------------------------------------------------------------------
# filter_repos
# ---------------------------------------------------------------------------


class TestFilterRepos(unittest.TestCase):
    """filter_repos() — build-tool exclusion logic."""

    def test_no_patterns_keeps_all_repos(self):
        repos = ["nlohmann_json", "googletest", "abseil-cpp"]
        self.assertEqual(filter_repos(repos, []), repos)

    def test_matching_pattern_excludes_repo(self):
        repos = ["cc_toolchain", "nlohmann_json"]
        result = filter_repos(repos, ["cc_toolchain"])
        self.assertNotIn("cc_toolchain", result)
        self.assertIn("nlohmann_json", result)

    def test_crate_index_repo_always_kept_even_when_pattern_matches(self):
        """crate_index__ repos are real dependencies and must never be filtered out."""
        repos = ["rules_rust++crate+crate_index__serde-1.0.228"]
        result = filter_repos(repos, ["rules_rust"])
        self.assertEqual(result, repos)

    def test_crates_io_prefix_always_kept(self):
        repos = ["crates_io__tokio-1.10.0"]
        result = filter_repos(repos, ["crates_io"])
        self.assertEqual(result, repos)

    def test_score_crates_always_kept(self):
        repos = ["score_crates__serde-1.0.0"]
        result = filter_repos(repos, ["score"])
        self.assertEqual(result, repos)

    def test_multiple_patterns_combined(self):
        repos = ["cc_toolchain", "rust_toolchain", "nlohmann_json"]
        result = filter_repos(repos, ["cc_toolchain", "rust_toolchain"])
        self.assertEqual(result, ["nlohmann_json"])

    def test_empty_repos(self):
        self.assertEqual(filter_repos([], ["pattern"]), [])

    def test_partial_pattern_match_excludes_repo(self):
        repos = ["score_cc_toolchain_linux", "my_lib"]
        result = filter_repos(repos, ["cc_toolchain"])
        self.assertNotIn("score_cc_toolchain_linux", result)
        self.assertIn("my_lib", result)


# ---------------------------------------------------------------------------
# resolve_component
# ---------------------------------------------------------------------------


class TestResolveComponentBazelDep(unittest.TestCase):
    """resolve_component() — bazel_dep module paths."""

    def _meta(self, **kwargs) -> dict:
        return {"modules": kwargs}

    def test_basic_bazel_dep_module(self):
        meta = self._meta(
            nlohmann_json={
                "version": "3.11.3",
                "purl": "pkg:generic/nlohmann_json@3.11.3",
                "license": "MIT",
                "supplier": "Niels Lohmann",
            }
        )
        comp = resolve_component("nlohmann_json", meta)
        self.assertIsNotNone(comp)
        self.assertEqual(comp["name"], "nlohmann_json")
        self.assertEqual(comp["version"], "3.11.3")
        self.assertEqual(comp["purl"], "pkg:generic/nlohmann_json@3.11.3")
        self.assertEqual(comp["license"], "MIT")
        self.assertEqual(comp["supplier"], "Niels Lohmann")

    def test_bzlmod_plus_suffix_stripped(self):
        """bzlmod appends '+' to repo names; the suffix must be stripped."""
        meta = self._meta(
            boost={
                "version": "1.87.0",
                "purl": "pkg:generic/boost@1.87.0",
                "license": "BSL-1.0",
            }
        )
        comp = resolve_component("boost+", meta)
        self.assertIsNotNone(comp)
        self.assertEqual(comp["name"], "boost")
        self.assertEqual(comp["version"], "1.87.0")

    def test_sha256_from_lockfile_becomes_checksum(self):
        """sha256 field from parse_module_lockfiles is surfaced as checksum."""
        meta = self._meta(
            boost={
                "version": "1.87.0",
                "purl": "pkg:generic/boost@1.87.0",
                "sha256": "abc123def456",
            }
        )
        comp = resolve_component("boost", meta)
        self.assertEqual(comp["checksum"], "abc123def456")

    def test_pedigree_fields_propagated(self):
        meta = self._meta(
            linux_kernel={
                "version": "5.10.130",
                "purl": "pkg:generic/linux_kernel@5.10.130",
                "pedigree_ancestors": ["pkg:generic/linux-kernel@5.10.0"],
                "pedigree_notes": "Backported CVE-2025-12345 fix",
            }
        )
        comp = resolve_component("linux_kernel", meta)
        self.assertEqual(
            comp["pedigree_ancestors"], ["pkg:generic/linux-kernel@5.10.0"]
        )
        self.assertEqual(comp["pedigree_notes"], "Backported CVE-2025-12345 fix")


class TestResolveComponentHttpArchive(unittest.TestCase):
    """resolve_component() — http_archive paths."""

    def _meta(self, **archives) -> dict:
        return {"modules": {}, "http_archives": archives}

    def test_http_archive_basic(self):
        meta = self._meta(
            linux_kernel={
                "version": "5.10.0",
                "purl": "pkg:generic/linux_kernel@5.10.0",
                "url": "https://example.com/linux.tar.gz",
                "license": "GPL-2.0-only",
                "sha256": "deadbeef1234",
            }
        )
        comp = resolve_component("linux_kernel", meta)
        self.assertIsNotNone(comp)
        self.assertEqual(comp["version"], "5.10.0")
        self.assertEqual(comp["license"], "GPL-2.0-only")
        self.assertEqual(comp["checksum"], "deadbeef1234")
        self.assertEqual(comp["url"], "https://example.com/linux.tar.gz")

    def test_http_archive_no_sha256_has_no_checksum_key(self):
        meta = self._meta(
            mylib={
                "version": "1.0",
                "purl": "pkg:generic/mylib@1.0",
            }
        )
        comp = resolve_component("mylib", meta)
        self.assertNotIn("checksum", comp)


class TestResolveComponentGitRepository(unittest.TestCase):
    """resolve_component() — git_repository paths."""

    def _meta(self, **repos) -> dict:
        return {"modules": {}, "http_archives": {}, "git_repositories": repos}

    def test_git_repository_basic(self):
        meta = self._meta(
            my_lib={
                "version": "abc1234",
                "purl": "pkg:generic/my_lib@abc1234",
                "remote": "https://github.com/example/my_lib",
                "license": "Apache-2.0",
            }
        )
        comp = resolve_component("my_lib", meta)
        self.assertIsNotNone(comp)
        self.assertEqual(comp["version"], "abc1234")
        self.assertEqual(comp["url"], "https://github.com/example/my_lib")

    def test_commit_date_used_when_version_unknown(self):
        """If version is 'unknown', commit_date provides the version string."""
        meta = self._meta(
            my_lib={
                "version": "unknown",
                "purl": "pkg:generic/my_lib@unknown",
                "remote": "https://github.com/example/my_lib",
                "commit_date": "2024-01-15",
            }
        )
        comp = resolve_component("my_lib", meta)
        self.assertEqual(comp["version"], "2024-01-15")

    def test_commit_date_not_used_when_version_known(self):
        meta = self._meta(
            my_lib={
                "version": "v2.0.0",
                "purl": "pkg:generic/my_lib@v2.0.0",
                "remote": "https://github.com/example/my_lib",
                "commit_date": "2024-01-15",
            }
        )
        comp = resolve_component("my_lib", meta)
        self.assertEqual(comp["version"], "v2.0.0")


class TestResolveComponentCrateCache(unittest.TestCase):
    """resolve_component() — metadata cache crate paths."""

    def _meta(self, **crates) -> dict:
        return {"modules": {}, "crates": crates}

    def test_crate_from_cache(self):
        meta = self._meta(
            my_crate={
                "version": "1.0.0",
                "purl": "pkg:cargo/my_crate@1.0.0",
                "license": "MIT",
                "description": "My crate",
                "supplier": "Me",
            }
        )
        comp = resolve_component("my_crate", meta)
        self.assertIsNotNone(comp)
        self.assertEqual(comp["version"], "1.0.0")
        self.assertEqual(comp["license"], "MIT")
        self.assertEqual(comp["purl"], "pkg:cargo/my_crate@1.0.0")

    def test_hyphen_to_underscore_lookup(self):
        """Bazel uses hyphens; Cargo.lock uses underscores — both must resolve."""
        meta = self._meta(
            my_crate={
                "version": "1.0.0",
                "purl": "pkg:cargo/my_crate@1.0.0",
                "license": "MIT",
            }
        )
        comp = resolve_component("my-crate", meta)
        self.assertIsNotNone(comp)
        self.assertEqual(comp["version"], "1.0.0")

    def test_crate_checksum_propagated(self):
        meta = self._meta(
            serde={
                "version": "1.0.228",
                "purl": "pkg:cargo/serde@1.0.228",
                "checksum": "abc123",
            }
        )
        comp = resolve_component("serde", meta)
        self.assertEqual(comp["checksum"], "abc123")


class TestResolveComponentCrateUniverse(unittest.TestCase):
    """resolve_component() — crate_universe bzlmod and legacy formats."""

    def _meta(self, **crates) -> dict:
        return {"modules": {}, "crates": crates}

    def test_bzlmod_format_simple_name(self):
        """rules_rust++crate+crate_index__serde-1.0.228 → serde 1.0.228."""
        meta = self._meta(
            serde={
                "license": "Apache-2.0 OR MIT",
                "description": "A serialization framework",
                "supplier": "David Tolnay",
            }
        )
        comp = resolve_component("rules_rust++crate+crate_index__serde-1.0.228", meta)
        self.assertIsNotNone(comp)
        self.assertEqual(comp["name"], "serde")
        self.assertEqual(comp["version"], "1.0.228")
        self.assertEqual(comp["purl"], "pkg:cargo/serde@1.0.228")
        self.assertEqual(comp["license"], "Apache-2.0 OR MIT")

    def test_bzlmod_format_complex_name_with_platform_suffix(self):
        """iceoryx2-bb-lock-free-qnx8-0.7.0 → name=iceoryx2-bb-lock-free-qnx8, version=0.7.0."""
        meta = self._meta()
        comp = resolve_component(
            "rules_rust++crate+crate_index__iceoryx2-bb-lock-free-qnx8-0.7.0", meta
        )
        self.assertIsNotNone(comp)
        self.assertEqual(comp["name"], "iceoryx2-bb-lock-free-qnx8")
        self.assertEqual(comp["version"], "0.7.0")
        self.assertEqual(comp["purl"], "pkg:cargo/iceoryx2-bb-lock-free-qnx8@0.7.0")

    def test_bzlmod_format_without_cache_entry_still_resolves(self):
        """Crate repos resolve even with no metadata cache entry."""
        meta = self._meta()
        comp = resolve_component("rules_rust++crate+crate_index__tokio-1.28.0", meta)
        self.assertIsNotNone(comp)
        self.assertEqual(comp["name"], "tokio")
        self.assertEqual(comp["version"], "1.28.0")

    def test_legacy_crates_io_format(self):
        """crates_io__tokio-1.10.0 → tokio 1.10.0."""
        meta = self._meta()
        comp = resolve_component("crates_io__tokio-1.10.0", meta)
        self.assertIsNotNone(comp)
        self.assertEqual(comp["name"], "tokio")
        self.assertEqual(comp["version"], "1.10.0")
        self.assertEqual(comp["purl"], "pkg:cargo/tokio@1.10.0")

    def test_legacy_format_metadata_enrichment(self):
        """Legacy crate repos pick up metadata from cache when available."""
        meta = self._meta(
            tokio={
                "license": "MIT",
                "description": "Async runtime",
                "supplier": "Tokio Contributors",
            }
        )
        comp = resolve_component("crates_io__tokio-1.10.0", meta)
        self.assertEqual(comp["license"], "MIT")
        self.assertEqual(comp["description"], "Async runtime")


class TestResolveComponentSpecialCases(unittest.TestCase):
    """resolve_component() — score_ prefix, dot sub-library, and unknown fallback."""

    def test_score_prefixed_repo(self):
        meta = {"modules": {}}
        comp = resolve_component("score_communication", meta)
        self.assertIsNotNone(comp)
        self.assertEqual(comp["name"], "score_communication")
        self.assertIn("eclipse-score", comp["purl"])
        self.assertEqual(comp["supplier"], "Eclipse Foundation")

    def test_dot_sub_library_inherits_from_parent_module(self):
        """boost.config+ must inherit version and license from the boost parent."""
        meta = {
            "modules": {
                "boost": {
                    "version": "1.87.0",
                    "purl": "pkg:generic/boost@1.87.0",
                    "license": "BSL-1.0",
                    "supplier": "Boost.org",
                    "sha256": "abc123",
                }
            }
        }
        comp = resolve_component("boost.config+", meta)
        self.assertIsNotNone(comp)
        self.assertEqual(comp["name"], "boost.config")
        self.assertEqual(comp["version"], "1.87.0")
        self.assertEqual(comp["license"], "BSL-1.0")
        self.assertEqual(comp["supplier"], "Boost.org")
        self.assertEqual(comp["checksum"], "abc123")

    def test_dot_sub_library_inherits_from_parent_http_archive(self):
        meta = {
            "modules": {},
            "http_archives": {
                "mylib": {
                    "version": "2.0.0",
                    "purl": "pkg:generic/mylib@2.0.0",
                    "license": "Apache-2.0",
                }
            },
        }
        comp = resolve_component("mylib.component+", meta)
        self.assertIsNotNone(comp)
        self.assertEqual(comp["version"], "2.0.0")
        self.assertEqual(comp["license"], "Apache-2.0")

    def test_unknown_repo_fallback(self):
        """Repos that match no known pattern return an 'unknown' placeholder."""
        meta = {"modules": {}}
        comp = resolve_component("some_unknown_lib", meta)
        self.assertIsNotNone(comp)
        self.assertEqual(comp["name"], "some_unknown_lib")
        self.assertEqual(comp["version"], "unknown")
        self.assertIn("some_unknown_lib", comp["purl"])

    def test_returns_dict_not_none_for_all_repo_types(self):
        """resolve_component always returns a dict, never None (all paths covered)."""
        meta = {
            "modules": {
                "boost": {"version": "1.87.0", "purl": "pkg:generic/boost@1.87.0"}
            },
            "http_archives": {},
            "git_repositories": {},
            "crates": {},
        }
        for repo_name in [
            "boost",
            "score_kyron",
            "boost.config+",
            "rules_rust++crate+crate_index__serde-1.0.228",
            "crates_io__tokio-1.10.0",
            "total_unknown_xyz",
        ]:
            with self.subTest(repo=repo_name):
                comp = resolve_component(repo_name, meta)
                self.assertIsNotNone(
                    comp, f"resolve_component returned None for {repo_name!r}"
                )
                self.assertIsInstance(comp, dict)


# ---------------------------------------------------------------------------
# deduplicate_components
# ---------------------------------------------------------------------------


class TestDeduplicateComponents(unittest.TestCase):
    """deduplicate_components() — dedup with metadata preference."""

    def test_no_duplicates_unchanged(self):
        components = [
            {"name": "serde", "version": "1.0.0"},
            {"name": "tokio", "version": "2.0.0"},
        ]
        result = deduplicate_components(components)
        self.assertEqual(len(result), 2)

    def test_exact_duplicate_keeps_first(self):
        components = [
            {"name": "serde", "version": "1.0.0"},
            {"name": "serde", "version": "1.0.0"},
        ]
        result = deduplicate_components(components)
        self.assertEqual(len(result), 1)

    def test_prefers_known_version_over_unknown(self):
        """When one entry has version='unknown' and the other has a real version, keep real."""
        components = [
            {"name": "serde", "version": "unknown"},
            {"name": "serde", "version": "1.0.228"},
        ]
        result = deduplicate_components(components)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["version"], "1.0.228")

    def test_prefers_entry_with_license(self):
        """When one entry has no license and the other does, keep the licensed one."""
        components = [
            {"name": "serde", "version": "1.0.0", "license": ""},
            {"name": "serde", "version": "1.0.0", "license": "MIT"},
        ]
        result = deduplicate_components(components)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["license"], "MIT")

    def test_empty_components(self):
        self.assertEqual(deduplicate_components([]), [])

    def test_three_duplicates_kept_correctly(self):
        components = [
            {"name": "foo", "version": "unknown", "license": ""},
            {"name": "foo", "version": "1.0", "license": ""},
            {"name": "foo", "version": "1.0", "license": "MIT"},
        ]
        result = deduplicate_components(components)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["version"], "1.0")


# ---------------------------------------------------------------------------
# parse_module_bazel_files
# ---------------------------------------------------------------------------


class TestParseModuleBazelFiles(unittest.TestCase):
    """parse_module_bazel_files() — MODULE.bazel version extraction."""

    def _write(self, content: str) -> str:
        fd, path = tempfile.mkstemp(suffix=".bazel")
        with os.fdopen(fd, "w") as f:
            f.write(content)
        self.addCleanup(os.unlink, path)
        return path

    def test_basic_extraction(self):
        path = self._write(
            'module(\n    name = "my_module",\n    version = "1.2.3",\n)'
        )
        result = parse_module_bazel_files([path])
        self.assertIn("my_module", result)
        self.assertEqual(result["my_module"]["version"], "1.2.3")
        self.assertEqual(result["my_module"]["purl"], "pkg:generic/my_module@1.2.3")

    def test_missing_file_gracefully_skipped(self):
        result = parse_module_bazel_files(["/nonexistent/path/MODULE.bazel"])
        self.assertEqual(result, {})

    def test_no_module_block_skipped(self):
        path = self._write("# no module() call here\n")
        result = parse_module_bazel_files([path])
        self.assertEqual(result, {})

    def test_multiple_files_merged(self):
        path_a = self._write('module(name = "lib_a", version = "1.0.0")')
        path_b = self._write('module(name = "lib_b", version = "2.0.0")')
        result = parse_module_bazel_files([path_a, path_b])
        self.assertIn("lib_a", result)
        self.assertEqual(result["lib_a"]["version"], "1.0.0")
        self.assertIn("lib_b", result)
        self.assertEqual(result["lib_b"]["version"], "2.0.0")

    def test_multiline_module_block(self):
        content = (
            "module(\n"
            '    name = "score_communication",\n'
            '    version = "0.3.0",\n'
            "    compatibility_level = 1,\n"
            ")\n"
        )
        path = self._write(content)
        result = parse_module_bazel_files([path])
        self.assertIn("score_communication", result)
        self.assertEqual(result["score_communication"]["version"], "0.3.0")

    def test_empty_list(self):
        result = parse_module_bazel_files([])
        self.assertEqual(result, {})


# ---------------------------------------------------------------------------
# parse_module_lockfiles
# ---------------------------------------------------------------------------


class TestParseModuleLockfiles(unittest.TestCase):
    """parse_module_lockfiles() — MODULE.bazel.lock version + checksum extraction."""

    def _write(self, data: dict) -> str:
        fd, path = tempfile.mkstemp(suffix=".lock")
        with os.fdopen(fd, "w") as f:
            json.dump(data, f)
        self.addCleanup(os.unlink, path)
        return path

    def test_basic_version_extraction(self):
        lockfile = {
            "registryFileHashes": {
                "https://bcr.bazel.build/modules/boost/1.87.0/MODULE.bazel": "sha256-abc",
            }
        }
        path = self._write(lockfile)
        result = parse_module_lockfiles([path])
        self.assertIn("boost", result)
        self.assertEqual(result["boost"]["version"], "1.87.0")
        self.assertEqual(result["boost"]["purl"], "pkg:generic/boost@1.87.0")

    def test_sha256_from_source_json(self):
        """source.json hash is surfaced as sha256 for CycloneDX hashes."""
        lockfile = {
            "registryFileHashes": {
                "https://bcr.bazel.build/modules/nlohmann_json/3.11.3/MODULE.bazel": "sha256-abc",
                "https://bcr.bazel.build/modules/nlohmann_json/3.11.3/source.json": "sha256-deadbeef",
            }
        }
        path = self._write(lockfile)
        result = parse_module_lockfiles([path])
        self.assertIn("nlohmann_json", result)
        self.assertEqual(result["nlohmann_json"]["sha256"], "sha256-deadbeef")

    def test_ambiguous_version_skipped(self):
        """Modules with more than one observed version are excluded to avoid guessing."""
        lockfile = {
            "registryFileHashes": {
                "https://bcr.bazel.build/modules/boost/1.83.0/MODULE.bazel": "sha256-a",
                "https://bcr.bazel.build/modules/boost/1.87.0/MODULE.bazel": "sha256-b",
            }
        }
        path = self._write(lockfile)
        result = parse_module_lockfiles([path])
        self.assertNotIn("boost", result)

    def test_missing_file_gracefully_skipped(self):
        result = parse_module_lockfiles(["/nonexistent/path/MODULE.bazel.lock"])
        self.assertEqual(result, {})

    def test_malformed_json_skipped(self):
        fd, path = tempfile.mkstemp(suffix=".lock")
        with os.fdopen(fd, "w") as f:
            f.write("not valid json {{{")
        self.addCleanup(os.unlink, path)
        result = parse_module_lockfiles([path])
        self.assertEqual(result, {})

    def test_empty_lockfile_skipped(self):
        path = self._write({})
        result = parse_module_lockfiles([path])
        self.assertEqual(result, {})

    def test_multiple_lockfiles_merged(self):
        lockfile_a = {
            "registryFileHashes": {
                "https://bcr.bazel.build/modules/boost/1.87.0/MODULE.bazel": "sha256-a",
            }
        }
        lockfile_b = {
            "registryFileHashes": {
                "https://bcr.bazel.build/modules/abseil-cpp/20230802.0/MODULE.bazel": "sha256-b",
            }
        }
        path_a = self._write(lockfile_a)
        path_b = self._write(lockfile_b)
        result = parse_module_lockfiles([path_a, path_b])
        self.assertIn("boost", result)
        self.assertIn("abseil-cpp", result)

    def test_version_purl_consistent(self):
        lockfile = {
            "registryFileHashes": {
                "https://bcr.bazel.build/modules/googletest/1.14.0/MODULE.bazel": "sha256-x",
            }
        }
        path = self._write(lockfile)
        result = parse_module_lockfiles([path])
        gt = result["googletest"]
        self.assertIn(gt["version"], gt["purl"])


# ---------------------------------------------------------------------------
# mark_missing_cpp_descriptions
# ---------------------------------------------------------------------------


class TestMarkMissingCppDescriptions(unittest.TestCase):
    """mark_missing_cpp_descriptions() — 'Missing' marker for C++ libs."""

    def test_library_without_description_marked_missing(self):
        """Non-Rust libraries with no description receive 'Missing' as placeholder."""
        components = [
            {
                "name": "nlohmann-json",
                "type": "library",
                "description": "",
                "purl": "pkg:generic/nlohmann-json@3.11.3",
            }
        ]
        result = mark_missing_cpp_descriptions(components)
        self.assertEqual(result[0]["description"], "Missing")

    def test_cargo_crate_not_marked_missing(self):
        """Rust crates (pkg:cargo/) must not receive 'Missing' — no cdxgen scan for them."""
        components = [
            {
                "name": "serde",
                "type": "library",
                "description": "",
                "purl": "pkg:cargo/serde@1.0.228",
            }
        ]
        result = mark_missing_cpp_descriptions(components)
        self.assertEqual(result[0]["description"], "")

    def test_existing_description_preserved(self):
        components = [
            {
                "name": "foo",
                "type": "library",
                "description": "JSON library",
                "purl": "pkg:generic/foo@1.0",
            }
        ]
        result = mark_missing_cpp_descriptions(components)
        self.assertEqual(result[0]["description"], "JSON library")

    def test_non_library_type_not_marked(self):
        """Applications and non-library types must not have 'Missing' injected."""
        components = [
            {
                "name": "myapp",
                "type": "application",
                "description": "",
                "purl": "pkg:generic/myapp@1.0",
            }
        ]
        result = mark_missing_cpp_descriptions(components)
        self.assertEqual(result[0]["description"], "")

    def test_mixed_components_handled_independently(self):
        components = [
            {
                "name": "cpp-lib",
                "type": "library",
                "description": "",
                "purl": "pkg:generic/cpp-lib@1.0",
            },
            {
                "name": "rust-crate",
                "type": "library",
                "description": "",
                "purl": "pkg:cargo/rust-crate@0.5",
            },
            {
                "name": "already-described",
                "type": "library",
                "description": "Has description",
                "purl": "pkg:generic/already-described@2.0",
            },
        ]
        result = mark_missing_cpp_descriptions(components)
        cpp = next(c for c in result if c["name"] == "cpp-lib")
        rust = next(c for c in result if c["name"] == "rust-crate")
        described = next(c for c in result if c["name"] == "already-described")
        self.assertEqual(cpp["description"], "Missing")
        self.assertEqual(rust["description"], "")
        self.assertEqual(described["description"], "Has description")


# ---------------------------------------------------------------------------
# main() — end-to-end integration
# ---------------------------------------------------------------------------


class TestMain(unittest.TestCase):
    """End-to-end integration tests for main(), covering the full SBOM pipeline."""

    _DEFAULT_CONFIG = {
        "component_name": "my_app",
        "component_version": "1.0.0",
        "producer_name": "Eclipse Foundation",
        "namespace": "https://eclipse.dev/score",
    }

    _DEFAULT_INPUT = {
        "external_repos": ["nlohmann_json"],
        "exclude_patterns": [],
        "config": _DEFAULT_CONFIG,
        "dep_module_files": [],
        "module_lockfiles": [],
        "external_dep_edges": [],
    }

    _DEFAULT_METADATA = {
        "modules": {
            "nlohmann_json": {
                "version": "3.11.3",
                "purl": "pkg:generic/nlohmann_json@3.11.3",
                "license": "MIT",
                "supplier": "Niels Lohmann",
            }
        }
    }

    def setUp(self):
        self.tmpdir = tempfile.mkdtemp()
        self._input_path = os.path.join(self.tmpdir, "input.json")
        self._metadata_path = os.path.join(self.tmpdir, "metadata.json")
        self._spdx_path = os.path.join(self.tmpdir, "output.spdx.json")
        self._cdx_path = os.path.join(self.tmpdir, "output.cdx.json")

    def tearDown(self):
        shutil.rmtree(self.tmpdir, ignore_errors=True)

    def _write_files(self, input_data=None, metadata=None):
        with open(self._input_path, "w") as f:
            json.dump(input_data if input_data is not None else self._DEFAULT_INPUT, f)
        with open(self._metadata_path, "w") as f:
            json.dump(metadata if metadata is not None else self._DEFAULT_METADATA, f)

    def _run(self, input_data=None, metadata=None, extra_args=None):
        """Write fixtures and run main(), returning the exit code."""
        self._write_files(input_data=input_data, metadata=metadata)
        argv = [
            "sbom_generator.py",
            "--input",
            self._input_path,
            "--metadata",
            self._metadata_path,
            "--spdx-output",
            self._spdx_path,
            "--cyclonedx-output",
            self._cdx_path,
        ]
        if extra_args:
            argv.extend(extra_args)
        with unittest.mock.patch("sys.argv", argv):
            return main()

    # -----------------------------------------------------------------------
    # Basic pipeline
    # -----------------------------------------------------------------------

    def test_returns_zero(self):
        self.assertEqual(self._run(), 0)

    def test_writes_valid_spdx(self):
        self._run()
        with open(self._spdx_path) as f:
            spdx = json.load(f)
        self.assertEqual(spdx["spdxVersion"], "SPDX-2.3")
        self.assertIn("packages", spdx)
        self.assertIn("relationships", spdx)

    def test_writes_valid_cyclonedx(self):
        self._run()
        with open(self._cdx_path) as f:
            cdx = json.load(f)
        self.assertEqual(cdx["bomFormat"], "CycloneDX")
        self.assertEqual(cdx["specVersion"], "1.6")
        self.assertIn("components", cdx)

    def test_component_appears_in_spdx(self):
        """A registered dependency appears as a package in SPDX output."""
        self._run()
        with open(self._spdx_path) as f:
            spdx = json.load(f)
        names = [p["name"] for p in spdx["packages"]]
        self.assertIn("nlohmann_json", names)

    # -----------------------------------------------------------------------
    # Root component filtering
    # -----------------------------------------------------------------------

    def test_root_component_not_in_deps(self):
        """component_name must not appear as a dependency in the SPDX output."""
        input_data = {
            **self._DEFAULT_INPUT,
            "external_repos": ["nlohmann_json", "my_app"],
        }
        self._run(input_data=input_data)
        with open(self._spdx_path) as f:
            spdx = json.load(f)
        dep_names = [
            p["name"]
            for p in spdx["packages"]
            if p.get("SPDXID") != "SPDXRef-RootPackage"
        ]
        self.assertNotIn("my_app", dep_names)

    # -----------------------------------------------------------------------
    # BCR known licenses
    # -----------------------------------------------------------------------

    def test_bcr_known_license_applied(self):
        """boost.* modules receive BSL-1.0 from BCR_KNOWN_LICENSES when no license is set."""
        input_data = {**self._DEFAULT_INPUT, "external_repos": ["boost.config+"]}
        metadata = {
            "modules": {
                "boost.config": {
                    "version": "1.83.0",
                    "purl": "pkg:generic/boost.config@1.83.0",
                }
            }
        }
        self._run(input_data=input_data, metadata=metadata)
        with open(self._spdx_path) as f:
            spdx = json.load(f)
        pkg = next(
            (p for p in spdx["packages"] if p.get("name") == "boost.config"), None
        )
        self.assertIsNotNone(pkg)
        self.assertIn("BSL-1.0", pkg.get("licenseConcluded", ""))

    # -----------------------------------------------------------------------
    # crate_universe repos
    # -----------------------------------------------------------------------

    def test_crate_universe_repo_resolves(self):
        """A bzlmod crate_universe repo resolves and appears as a package in SPDX output."""
        repo = "rules_rust++crate+crate_index__serde-1.0.228"
        input_data = {**self._DEFAULT_INPUT, "external_repos": [repo]}
        metadata = {
            "crates": {
                "serde": {
                    "version": "1.0.228",
                    "purl": "pkg:cargo/serde@1.0.228",
                    "license": "MIT OR Apache-2.0",
                }
            }
        }
        self._run(input_data=input_data, metadata=metadata)
        with open(self._spdx_path) as f:
            spdx = json.load(f)
        names = [p["name"] for p in spdx["packages"]]
        self.assertIn("serde", names)

    # -----------------------------------------------------------------------
    # Exclude patterns
    # -----------------------------------------------------------------------

    def test_exclude_patterns_remove_repos(self):
        """Repos matching exclude_patterns are absent from SPDX output."""
        input_data = {
            "external_repos": ["nlohmann_json", "cc_toolchain"],
            "exclude_patterns": ["cc_toolchain"],
            "config": self._DEFAULT_CONFIG,
            "dep_module_files": [],
            "module_lockfiles": [],
            "external_dep_edges": [],
        }
        self._run(input_data=input_data)
        with open(self._spdx_path) as f:
            spdx = json.load(f)
        names = [p["name"] for p in spdx["packages"]]
        self.assertNotIn("cc_toolchain", names)

    # -----------------------------------------------------------------------
    # Auto-detected component version
    # -----------------------------------------------------------------------

    def test_auto_detect_component_version(self):
        """component_version is inferred from metadata.modules when absent from config."""
        config = {
            "component_name": "my_app",
            "producer_name": "Eclipse Foundation",
            "namespace": "https://eclipse.dev/score",
        }
        input_data = {**self._DEFAULT_INPUT, "config": config, "external_repos": []}
        metadata = {
            "modules": {
                "my_app": {"version": "2.5.0", "purl": "pkg:generic/my_app@2.5.0"}
            }
        }
        self._run(input_data=input_data, metadata=metadata)
        with open(self._spdx_path) as f:
            spdx = json.load(f)
        root_pkg = next(
            p for p in spdx["packages"] if p.get("SPDXID") == "SPDXRef-RootPackage"
        )
        self.assertEqual(root_pkg["versionInfo"], "2.5.0")

    # -----------------------------------------------------------------------
    # dep_module_files
    # -----------------------------------------------------------------------

    def test_dep_module_files_version_in_output(self):
        """Versions parsed from dep_module_files appear in the SPDX packages."""
        module_bazel = os.path.join(self.tmpdir, "dep_MODULE.bazel")
        with open(module_bazel, "w") as f:
            f.write('module(name = "zlib", version = "1.3.1")\n')
        input_data = {
            "external_repos": ["zlib"],
            "exclude_patterns": [],
            "config": self._DEFAULT_CONFIG,
            "dep_module_files": [module_bazel],
            "module_lockfiles": [],
            "external_dep_edges": [],
        }
        self._run(input_data=input_data, metadata={})
        with open(self._spdx_path) as f:
            spdx = json.load(f)
        zlib_pkg = next((p for p in spdx["packages"] if p.get("name") == "zlib"), None)
        self.assertIsNotNone(zlib_pkg)
        self.assertEqual(zlib_pkg.get("versionInfo"), "1.3.1")

    # -----------------------------------------------------------------------
    # module_lockfiles
    # -----------------------------------------------------------------------

    def test_module_lockfiles_version_in_output(self):
        """Versions extracted from MODULE.bazel.lock appear in SPDX packages."""
        lockfile = os.path.join(self.tmpdir, "MODULE.bazel.lock")
        lock_data = {
            "registryFileHashes": {
                "https://bcr.bazel.build/modules/zlib/1.3.1/MODULE.bazel": "sha256:abc"
            }
        }
        with open(lockfile, "w") as f:
            json.dump(lock_data, f)
        input_data = {
            "external_repos": ["zlib"],
            "exclude_patterns": [],
            "config": self._DEFAULT_CONFIG,
            "dep_module_files": [],
            "module_lockfiles": [lockfile],
            "external_dep_edges": [],
        }
        self._run(input_data=input_data, metadata={})
        with open(self._spdx_path) as f:
            spdx = json.load(f)
        zlib_pkg = next((p for p in spdx["packages"] if p.get("name") == "zlib"), None)
        self.assertIsNotNone(zlib_pkg)
        self.assertEqual(zlib_pkg.get("versionInfo"), "1.3.1")

    # -----------------------------------------------------------------------
    # --crates-cache
    # -----------------------------------------------------------------------

    def test_crates_cache_enriches_crate(self):
        """--crates-cache provides license and version data for resolved crate repos."""
        cache = {
            "serde": {
                "version": "1.0.228",
                "purl": "pkg:cargo/serde@1.0.228",
                "license": "MIT OR Apache-2.0",
                "description": "A serialization framework",
            }
        }
        cache_path = os.path.join(self.tmpdir, "crates_cache.json")
        with open(cache_path, "w") as f:
            json.dump(cache, f)
        repo = "rules_rust++crate+crate_index__serde-1.0.228"
        input_data = {**self._DEFAULT_INPUT, "external_repos": [repo]}
        self._run(
            input_data=input_data,
            metadata={},
            extra_args=["--crates-cache", cache_path],
        )
        with open(self._spdx_path) as f:
            spdx = json.load(f)
        serde_pkg = next(
            (p for p in spdx["packages"] if p.get("name") == "serde"), None
        )
        self.assertIsNotNone(serde_pkg)
        self.assertEqual(serde_pkg.get("versionInfo"), "1.0.228")

    # -----------------------------------------------------------------------
    # --cdxgen-sbom
    # -----------------------------------------------------------------------

    def test_cdxgen_sbom_enriches_cpp_description(self):
        """--cdxgen-sbom fills in description for C++ components from cdxgen data."""
        cdxgen = {
            "bomFormat": "CycloneDX",
            "specVersion": "1.6",
            "components": [
                {
                    "name": "nlohmann_json",
                    "version": "3.11.3",
                    "purl": "pkg:generic/nlohmann_json@3.11.3",
                    "licenses": [{"license": {"id": "MIT"}}],
                    "description": "JSON for Modern C++",
                }
            ],
        }
        cdxgen_path = os.path.join(self.tmpdir, "cdxgen.cdx.json")
        with open(cdxgen_path, "w") as f:
            json.dump(cdxgen, f)
        metadata = {
            "modules": {
                "nlohmann_json": {
                    "version": "3.11.3",
                    "purl": "pkg:generic/nlohmann_json@3.11.3",
                    "license": "MIT",
                }
            }
        }
        self._run(metadata=metadata, extra_args=["--cdxgen-sbom", cdxgen_path])
        with open(self._cdx_path) as f:
            cdx = json.load(f)
        comp = next(
            (c for c in cdx["components"] if c.get("name") == "nlohmann_json"), None
        )
        self.assertIsNotNone(comp)
        self.assertEqual(comp.get("description"), "JSON for Modern C++")

    # -----------------------------------------------------------------------
    # Output file selection
    # -----------------------------------------------------------------------

    def test_only_spdx_output_does_not_create_cdx(self):
        """Passing only --spdx-output must not create a CycloneDX file."""
        self._write_files()
        argv = [
            "sbom_generator.py",
            "--input",
            self._input_path,
            "--metadata",
            self._metadata_path,
            "--spdx-output",
            self._spdx_path,
        ]
        with unittest.mock.patch("sys.argv", argv):
            rc = main()
        self.assertEqual(rc, 0)
        self.assertTrue(os.path.exists(self._spdx_path))
        self.assertFalse(os.path.exists(self._cdx_path))

    def test_only_cdx_output_does_not_create_spdx(self):
        """Passing only --cyclonedx-output must not create an SPDX file."""
        self._write_files()
        argv = [
            "sbom_generator.py",
            "--input",
            self._input_path,
            "--metadata",
            self._metadata_path,
            "--cyclonedx-output",
            self._cdx_path,
        ]
        with unittest.mock.patch("sys.argv", argv):
            rc = main()
        self.assertEqual(rc, 0)
        self.assertTrue(os.path.exists(self._cdx_path))
        self.assertFalse(os.path.exists(self._spdx_path))
