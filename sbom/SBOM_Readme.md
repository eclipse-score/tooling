# SBOM Setup Guide

## 1. Configure MODULE.bazel

Add the SBOM metadata extension in your **root** MODULE.bazel (e.g. `reference_integration/MODULE.bazel`):

```starlark
# Enable SBOM metadata collection from all modules in the dependency graph
sbom_ext = use_extension("@score_tooling//sbom:extensions.bzl", "sbom_metadata")
use_repo(sbom_ext, "sbom_metadata")
```

**For modules using `local_path_override` or `git_override`**, also add a `track_module` tag for each such module. Without this, their versions cannot be auto-detected and will appear as `unknown` in the SBOM:

```starlark
# Required for modules with local_path_override or git_override (no registry version)
sbom_ext.track_module(name = "score_baselibs")
sbom_ext.track_module(name = "score_communication")
sbom_ext.track_module(name = "score_orchestrator")
# ... one entry per overridden module
```

No manual license entries are needed — all license metadata is collected automatically.

## 2. Add SBOM Target in BUILD

```starlark
load("@score_tooling//sbom:defs.bzl", "sbom")

sbom(
    name = "my_sbom",
    targets = ["//my/app:binary"],
    component_name = "my_application",
    component_version = "1.0.0",
    # Rust crate metadata from multiple MODULE.bazel.lock files
    module_lockfiles = [
        "@score_crates//:MODULE.bazel.lock",
        ":MODULE.bazel.lock",  # workspace's own lockfile for additional crates
    ],
    auto_crates_cache = True,
    auto_cdxgen = True,  # Requires system-installed npm/cdxgen (see below)
)
```

### Parameters

| Parameter | Default | Description |
| :--- | :--- | :--- |
| `targets` | _(required)_ | Bazel targets to include in SBOM |
| `component_name` | rule name | Main component name |
| `component_version` | `""` | Version string |
| `output_formats` | `["spdx", "cyclonedx"]` | Output formats: `"spdx"` and/or `"cyclonedx"` |
| `module_lockfiles` | `[]` | List of MODULE.bazel.lock files for Rust crate metadata. Pass `@score_crates//:MODULE.bazel.lock` (centralized crate specs) and `:MODULE.bazel.lock` (workspace-local crates). Each lockfile is parsed for crate name, version, and sha256. |
| `cargo_lockfile` | `None` | Optional Cargo.lock for additional crates. Usually not needed when `module_lockfiles` covers all crates. |
| `auto_crates_cache` | `True` | Auto-generate crates cache when `module_lockfiles` or `cargo_lockfile` is set |
| `auto_cdxgen` | `False` | Auto-run cdxgen when no `cdxgen_sbom` is provided |
| `cdxgen_sbom` | `None` | Label to a pre-generated CycloneDX JSON from cdxgen for C++ enrichment |
| `producer_name` | `"Eclipse Foundation"` | SBOM producer organization name (appears in `metadata.supplier`) |
| `producer_url` | `"https://projects.eclipse.org/projects/automotive.score"` | SBOM producer URL |
| `sbom_authors` | `[]` | Author strings for `metadata.authors` (e.g. `["Eclipse SCORE Team"]`) |
| `generation_context` | `""` | Lifecycle phase: `"pre-build"`, `"build"`, or `"post-build"` |
| `sbom_tools` | `[]` | Additional tool names added to `metadata.tools` |
| `namespace` | `"https://eclipse.dev/score"` | Base URI for the SPDX document namespace |
| `exclude_patterns` | _(build tools)_ | List of repo name substrings to exclude (e.g. `rules_rust`, `bazel_tools`). Defaults exclude common Bazel build-tool repos. |
| `dep_module_files` | `[]` | Additional MODULE.bazel files from dependency modules for version extraction |

## 3. Install Prerequisites

### For `auto_crates_cache` (Rust crate metadata)

License data for Rust crates is fetched via [dash-license-scan](https://github.com/eclipse-score/dash-license-scan). Description and supplier metadata is fetched from the crates.io API (parallel, ~10 concurrent requests). Requires:

```bash
# Install uv (Python package runner)
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install Java >= 11 (required by Eclipse dash-licenses JAR)
# Option 1: Ubuntu/Debian
sudo apt install openjdk-11-jre-headless

# Option 2: Fedora/RHEL
sudo dnf install java-11-openjdk-headless

# Verify installation
uvx dash-license-scan --help
java -version
```

### For `auto_cdxgen` (C++ dependency scanning)

If using `auto_cdxgen = True` to automatically scan C++ dependencies:

```bash
# Install Node.js and cdxgen globally
# Option 1: Using nvm (recommended)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
source ~/.bashrc
nvm install 20
npm install -g @cyclonedx/cdxgen

# Verify installation
which cdxgen
cdxgen --version
```

**Note:** If you don't have npm/cdxgen installed, set `auto_cdxgen = False` in your SBOM configuration.
When `auto_cdxgen` is enabled, the SBOM rule runs cdxgen against the repository path of the selected Bazel targets (for example `external/score_baselibs+` for `@score_baselibs//...` targets).

## 4. Build

```bash
bazel build //:my_sbom
```

## 5. Output

Generated files in `bazel-bin/`:

- `my_sbom.spdx.json` — SPDX 2.3 format
- `my_sbom.cdx.json` — CycloneDX 1.6 format
- `my_sbom_crates_metadata.json` — Auto-generated Rust crate cache (if `auto_crates_cache = True`)
- `my_sbom_cdxgen.cdx.json` — C++ dependencies from cdxgen (if `auto_cdxgen = True`)

---

## Toolchain Components

### Core Tools

| Tool | Role | Required For |
|------|------|--------------|
| [Bazel](https://bazel.build) | Build system — rules, aspects, and module extensions drive dependency discovery and SBOM generation | All SBOM generation |
| [Python 3](https://www.python.org) | Runtime for the SBOM generator, formatters, and metadata extraction scripts | All SBOM generation |
| [dash-license-scan](https://github.com/eclipse-score/dash-license-scan) | Rust crate license metadata via Eclipse Foundation + ClearlyDefined | Rust metadata extraction when `auto_crates_cache = True` |
| [uv / uvx](https://docs.astral.sh/uv/) | Python package runner for dash-license-scan | Rust metadata extraction when `auto_crates_cache = True` |
| [Java >= 11](https://openjdk.org) | Runtime for Eclipse dash-licenses JAR (used by dash-license-scan) | Rust metadata extraction when `auto_crates_cache = True` |
| [crates.io API](https://crates.io) | Description and supplier metadata for Rust crates (parallel fetching) | Rust metadata extraction when `auto_crates_cache = True` |
| [@cyclonedx/cdxgen](https://github.com/CycloneDX/cdxgen) | C++ dependency scanner and license discovery tool | C++ metadata extraction when `auto_cdxgen = True` |
| [Node.js / npm](https://nodejs.org) | Runtime for cdxgen | C++ metadata extraction when `auto_cdxgen = True` |

### Architecture

```
                        ┌──────────────────┐
                        │   Bazel build    │
                        └────────┬─────────┘
                                 │
                 ┌───────────────┼───────────────┐
                 │               │               │
                 v               v               v
          MODULE.bazel     Bazel targets    Lockfiles
                 │               │               │
                 v               v               v
          metadata.json    _deps.json      License + metadata
        (module versions)  (dep graph,     (dash-license-scan
                          dep edges)      + crates.io API
                 │               │           + cdxgen)
                 └───────────────┼───────────────┘
                                 │
                                 v
                        ┌──────────────────┐
                        │ sbom_generator   │
                        │ (match & resolve)│
                        └────────┬─────────┘
                                 │
                        ┌────────┴────────┐
                        v                 v
                   .spdx.json        .cdx.json
```

**Data sources:**
- **Bazel module graph** — version, PURL, and registry info for `bazel_dep` modules
- **Bazel aspect** — transitive dependency graph and external repo dependency edges
- **dash-license-scan** — Rust crate licenses via Eclipse Foundation + ClearlyDefined (from MODULE.bazel.lock or Cargo.lock)
- **crates.io API** — description and supplier for Rust crates (supplier extracted from GitHub repository URL)
- **cdxgen** — C++ dependency licenses, descriptions, and suppliers (from source tree scan)

### Automatically Populated Fields

The following SBOM fields are populated automatically without manual configuration:

| Field | Rust Crates | C++ Dependencies | Bazel Modules |
|-------|-------------|------------------|---------------|
| License | dash-license-scan | cdxgen | — |
| Description | crates.io API | cdxgen (falls back to `"Missing"` when unavailable) | — |
| Supplier | crates.io API (GitHub org from repository URL) | cdxgen | — |
| Version | MODULE.bazel.lock / Cargo.lock | cdxgen (with MODULE.bazel.lock fallback for Bazel modules) | Bazel module graph |
| Checksum (SHA-256) | MODULE.bazel.lock / Cargo.lock | BCR `source.json` `sha256` + cdxgen `hashes` (when present) | http_archive `sha256` + MODULE.bazel.lock BCR `source.json` |
| PURL | Auto-generated (`pkg:cargo/...`) | cdxgen | Auto-generated |

### Platform-Specific Crate Handling

Crates with platform-specific suffixes (e.g. `iceoryx2-bb-lock-free-qnx8`) that don't exist on crates.io are handled by stripping the suffix and falling back to the base crate name for description and supplier lookup.

### What Is Excluded from SBOM

- Dependencies not in the transitive dep graph of your `targets`
- Build toolchain repos matching `exclude_patterns` (e.g. `rules_rust`, `rules_cc`, `bazel_tools`, `platforms`)

## Example

See [reference_integration/BUILD](../../reference_integration/BUILD) for working SBOM targets and [reference_integration/MODULE.bazel](../../reference_integration/MODULE.bazel) for the metadata extension setup.

Each SBOM target uses `module_lockfiles` to provide crate version/checksum data from multiple lockfiles and `auto_crates_cache = True` to automatically fetch license, description, and supplier data.

### score_crates Integration

The `score_crates` module provides centralized Rust crate management for the SCORE project. Its `MODULE.bazel.lock` file contains the majority of resolved crate specs (name, version, sha256) generated by `cargo-bazel`. The workspace's own `MODULE.bazel.lock` may contain additional crates not in `score_crates`. Both lockfiles should be passed via `module_lockfiles` to ensure complete coverage.

## CISA 2025 Element Coverage (CycloneDX)

The table below maps the CISA 2025 draft elements to CycloneDX fields and notes current support in this SBOM generator.

| CISA 2025 Element | CycloneDX Field (JSON) | Support | Notes |
|---|---|---|---|
| Software Producer | `components[].supplier.name` | **Supported** | Root producer is set in `metadata.component.supplier`. For components, supplier is auto-extracted from crates.io repository URL (Rust) or from cdxgen (C++); in the current baselibs example, Boost BCR modules have no supplier because cdxgen does not provide one. |
| Component Name | `components[].name` | **Supported** | Single name; aliases are stored as `properties` with `cdx:alias`. |
| Component Version | `components[].version` | **Supported** | If unknown and source is git repo with `commit_date`, version can fall back to that date. |
| Software Identifiers | `components[].purl`, `components[].cpe` | **Supported (PURL)** / **Optional (CPE)** | PURL is generated for all components. CPE is optional if provided in metadata. |
| Component Hash | `components[].hashes` | **Supported** | SHA-256 is populated for Rust crates (from lockfiles) and for BCR / http_archive / some cdxgen-backed C++ components. In the current examples, Rust crates and Boost BCR modules have hashes; some QNX-specific crates and other C++ deps may not. |
| License | `components[].licenses` | **Supported (Rust) / Best-effort (C++)** | Rust licenses are auto-fetched via dash-license-scan and are present for most crates (e.g. Kyron SBOM); some crates like `iceoryx2-*` may still lack licenses. For C++ components, licenses are only present when cdxgen (or an upstream SBOM) provides them; in the current baselibs example, Boost BCR modules have empty `licenses`. Compound SPDX expressions (AND/OR) use the `expression` field per CycloneDX spec. |
| Component Description | `components[].description` | **Supported** | Auto-fetched from crates.io API (Rust) and cdxgen (C++), with C++ falling back to `"Missing"` when no description is available (as seen for Boost in the baselibs SBOM). |
| Dependency Relationship | `dependencies` | **Supported** | Uses external repo dependency edges from Bazel aspect; both Kyron and baselibs SBOMs include a dependency graph for the root component. |
| Pedigree / Derivation | `components[].pedigree` | **Supported (manual)** | Must be provided via `sbom_ext.license()` with `pedigree_*` fields. Not auto-deduced. |
| SBOM Author | `metadata.authors` | **Supported** | Set via `sbom_authors` in `sbom()` rule (e.g. `"Eclipse SCORE Team"` in the examples). |
| Tool Name | `metadata.tools` | **Supported** | Always includes `score-sbom-generator`; extra tools can be added via `sbom_tools`. |
| Timestamp | `metadata.timestamp` | **Supported** | ISO 8601 UTC timestamp generated at build time. |
| Generation Context | `metadata.lifecycles` | **Supported** | Set via `generation_context` in `sbom()` rule (`pre-build`, `build`, `post-build`). |

### SPDX-Specific Notes

- **LicenseRef-* declarations**: Any `LicenseRef-*` identifiers used in license fields are automatically declared in `hasExtractedLicensingInfos` as required by SPDX 2.3.
- **Supplier**: Emitted as `Organization: <name>` in the SPDX `supplier` field.

### Notes on Missing Data
If a field is absent in output, it usually means the source metadata was not provided:
- Licenses and suppliers are auto-populated from dash-license-scan (Rust) or cdxgen (C++). For C++ dependencies, licenses and suppliers are available only when cdxgen can resolve the component; Bazel Central Registry modules like `boost.*` may have empty licenses if cdxgen cannot infer them.
- CPE, aliases, and pedigree are optional and must be explicitly set via `sbom_ext.license()`.
- Rust crate licenses require a crates metadata cache; this is generated automatically when `module_lockfiles` (or `cargo_lockfile`) is provided to `sbom()`. License data is fetched via `dash-license-scan` (Eclipse Foundation + ClearlyDefined). The `score_crates` MODULE.bazel.lock combined with the workspace's MODULE.bazel.lock provides complete coverage.
- If cdxgen cannot resolve C++ package metadata for a Bazel-only dependency graph, SBOM generation sets C++ dependency descriptions to `"Missing"`.

Examples (add to `MODULE.bazel`):

```starlark
# Optional metadata (CPE, aliases, pedigree)
# Note: sbom_ext.license() should only be used for pedigree, CPE, and aliases.
# Licenses and suppliers are auto-populated from dash-license-scan (Rust) or cdxgen (C++).
sbom_ext.license(
    name = "linux-kernel",
    cpe = "cpe:2.3:o:linux:linux_kernel:*:*:*:*:*:*:*:*",
    aliases = ["linux", "kernel"],
    pedigree_ancestors = ["pkg:generic/linux-kernel@5.10.130"],
    pedigree_notes = "Backported CVE-2025-12345 fix from 5.10.130",
)
```

### C++ license data and dash-license-scan

- **Rust crates**  
  Rust licenses are obtained via `generate_crates_metadata_cache.py`, which reads `MODULE.bazel.lock` / `Cargo.lock`, builds a synthetic `Cargo.lock`, runs `uvx dash-license-scan` (backed by Eclipse dash-licenses), and writes a `crates_metadata.json` cache that `sbom_generator.py` consumes.

- **C++ dependencies**
  C++ licenses and suppliers are resolved through two mechanisms:

  1. **cdxgen scan** — when `auto_cdxgen = True` (or a `cdxgen_sbom` label is provided), cdxgen scans the source tree for C++ package metadata. This is the primary automated source for C++ license, supplier, version, and PURL.

  2. **`cpp_metadata.json` cache** — populated by running `generate_cpp_metadata_cache.py` against cdxgen output. **This file must always be generated by the script, never edited by hand.** See the no-manual-fallback requirement below.

  There is currently **no dash-license-scan integration for C++ SBOMs**. `dash-license-scan` understands purls like `pkg:cargo/...`, `pkg:pypi/...`, `pkg:npm/...`, and `pkg:maven/...`, but not `pkg:generic/...` (used for BCR modules), so running it on the C++ CycloneDX SBOM does not improve C++ license coverage.

### No-manual-fallback requirement (MUST)

**All SBOM fields must originate from automated sources. No manually-curated fallback values are permitted for any field — not checksum, not license, not supplier, not version, not PURL, not description.**

This applies to every data source in the pipeline:

| Source | Status | What it provides |
|---|---|---|
| `MODULE.bazel.lock` `source.json` sha256 | ✅ Automated | Checksum for BCR C++ modules |
| `http_archive sha256 =` field | ✅ Automated | Checksum for non-BCR deps |
| cdxgen source-tree scan | ✅ Automated | License, supplier, version, PURL for C++ |
| `generate_cpp_metadata_cache.py` output | ✅ Automated (generated from cdxgen) | Persistent C++ metadata cache |
| dash-license-scan | ✅ Automated | License for Rust crates |
| `cpp_metadata.json` with hand-written entries | ❌ **Forbidden** | — |
| `BCR_KNOWN_LICENSES` dict in `sbom_generator.py` | ⚠️ Known violation — must be removed | License/supplier for BCR C++ modules |

**Why:** A manually-written value is version-pinned to whatever version string happens to be in the file at the time of writing. If the workspace resolves a different version of that component, the value silently describes the wrong artifact. An absent field is honest and correct; a manually-guessed field is a compliance violation and a traceability lie.

**Correct behaviour for missing data:** If an automated source cannot determine a field, the field is absent in the SBOM output. This is expected and acceptable.

**Enforcement:** `test_cpp_enrich_checksum.py::TestNoManualFallbackInCppMetadata` asserts that `cpp_metadata.json` is empty and contains no SBOM fields. If entries are needed, regenerate the file:

```bash
npx @cyclonedx/cdxgen -t cpp --deep -r -o cdxgen_output.cdx.json
python3 tooling/sbom/scripts/generate_cpp_metadata_cache.py \
    cdxgen_output.cdx.json tooling/sbom/cpp_metadata.json
```

**Known violation — `BCR_KNOWN_LICENSES`:** The `BCR_KNOWN_LICENSES` dict hardcoded in `sbom_generator.py` is a manually-maintained license/supplier table for Bazel Central Registry C++ modules. It violates this requirement and must be replaced with automated BCR metadata fetching (e.g. querying the BCR `MODULE.bazel` or `metadata.json` at build time). Until that is implemented, BCR C++ modules that cdxgen cannot resolve will have missing license fields in the SBOM — which is the correct, honest output.

---

## SPDX Version Decision (stay on 2.3)

This generator emits **SPDX 2.3** and will not migrate to SPDX 3.0 until tooling support matures.

### Why not SPDX 3.0?

SPDX 3.0 is a **breaking rewrite**, not an additive update:

| Aspect | SPDX 2.3 | SPDX 3.0 |
|---|---|---|
| Serialization | Flat JSON | JSON-LD (`@context` + `@graph`) |
| Top-level key | `spdxVersion: "SPDX-2.3"` | `@context: "https://spdx.org/rdf/3.0.1/spdx-context.jsonld"` |
| Package fields | `versionInfo`, `licenseConcluded`, `SPDXID` | `software_packageVersion`, licensing profile objects, `spdxId` |
| Relationships | Array in document | Standalone elements in `@graph` |
| Profiles | None | Mandatory `profileConformance` declaration |

**Downstream consumer support as of Feb 2026 — tools that read/process our SBOM output, none support SPDX 3.0:**

| Tool | SPDX 2.3 | SPDX 3.0 |
|---|---|---|
| GitHub Dependabot / Dependency Submission API | ✅ SPDX 2.3 (export) / action works with 2.3 in practice | ❌ |
| Trivy | ✅ generates 2.3 | ❌ |
| Grype | ✅ consumes 2.x | ❌ |
| Syft | ✅ generates 2.3 | ❌ |
| spdx-tools (Python) | ✅ full support | ⚠️ "experimental, unstable" |

The `spdx-tools` Python library (latest: v0.8.4, Jan 2025) still describes its SPDX 3.0 support as "neither complete nor stable" and explicitly warns against production use. v0.8.4 added Python 3.14 support but made no SPDX 3.0 improvements.

For SCORE's use case (license data, PURL, checksums, dependency graph), SPDX 2.3 covers all requirements with zero compatibility issues.

### Revisit trigger

Reconsider migration when **Trivy or GitHub Dependabot** announces production SPDX 3.0 support. At that point the required changes are:

- `tooling/sbom/internal/generator/spdx_formatter.py` — full rewrite (flat JSON → JSON-LD `@graph`, new field names)
- `tooling/sbom/tests/test_spdx_formatter.py` — all 17 tests need rewriting
- `tooling/sbom/scripts/spdx_to_github_snapshot.py` — relationship and `externalRefs` parsing

