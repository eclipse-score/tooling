# About

SBOM tooling gives a set of bazel rules that generates a Software Bill of Materials
in SPDX 2.3 and CycloneDX 1.6 format for a given Bazel target.

# Setup

## 1. Configure MODULE.bazel

Add the SBOM metadata extension in your **root** MODULE.bazel:

```starlark
sbom_ext = use_extension("@score_tooling//sbom:extensions.bzl", "sbom_metadata")
use_repo(sbom_ext, "sbom_metadata")
```

**For modules using `local_path_override` or `git_override`**, also add a `track_module` tag for each such module. Without this, their versions cannot be auto-detected and will appear as `unknown` in the SBOM. The extension reads the version directly from the module's own `MODULE.bazel` file:

```starlark
sbom_ext = use_extension("@score_tooling//sbom:extensions.bzl", "sbom_metadata")
sbom_ext.track_module(name = "score_baselibs")
sbom_ext.track_module(name = "score_kyron")
use_repo(sbom_ext, "sbom_metadata")
```

## 2. Add SBOM Target in BUILD

```starlark
load("@score_tooling//sbom:defs.bzl", "sbom")

sbom(
    name = "my_sbom",
    targets = ["//my/app:binary"],
    component_name = "my_application",
    component_version = "1.0.0",
    module_lockfiles = [
        "@score_crates//:MODULE.bazel.lock",
        ":MODULE.bazel.lock",
    ],
    auto_crates_cache = True,
    auto_cdxgen = True,
)
```

### Parameters

| Parameter | Default | Description |
|---|---|---|
| `name` | *(required)* | Rule name; also used as the output filename prefix (e.g. `my_sbom` → `my_sbom.spdx.json`). |
| `targets` | *(required)* | Bazel targets whose transitive dependencies are included in the SBOM. |
| `component_name` | rule `name` | Name of the root component written into the SBOM; defaults to the rule name if omitted. |
| `component_version` | `None` | Version string for the root component; auto-detected from the module graph when omitted. |
| `module_lockfiles` | `[]` | One or more `MODULE.bazel.lock` files used to extract dependency versions and SHA-256 checksums; C++ projects need only the workspace lockfile (`:MODULE.bazel.lock`), Rust projects should also pass `@score_crates//:MODULE.bazel.lock` to cover crate versions and checksums. |
| `auto_crates_cache` | `True` | Runs `generate_crates_metadata_cache` at build time (requires network) to fetch Rust crate license and supplier data from dash-license-scan and crates.io; set to `False` only as a workaround for air-gapped or offline build environments — doing so produces a non-compliant SBOM where all Rust crates show `NOASSERTION` for license, supplier, and description. Has no effect when no lockfiles are provided (pure C++ projects). |
| `cargo_lockfile` | `None` | Path to a `Cargo.lock` file for crate enumeration; not needed when `module_lockfiles` is provided, as a synthetic `Cargo.lock` is generated from it automatically. **Deprecated — will be removed in a future release.** |
| `cdxgen_sbom` | `None` | Label to a pre-generated cdxgen CycloneDX JSON file; alternative to `auto_cdxgen` for C++ projects where cdxgen cannot run inside the Bazel build (e.g. CI environment without npm). Run cdxgen manually and pass its output here. Ignored for pure Rust projects. |
| `auto_cdxgen` | `False` | Runs cdxgen automatically inside the Bazel build (requires npm + `@cyclonedx/cdxgen` installed on the build machine); alternative to `cdxgen_sbom` for C++ projects. Uses `no-sandbox` execution to scan the source tree. Ignored for pure Rust projects. |
| `output_formats` | `["spdx", "cyclonedx"]` | List of output formats to generate; valid values are `"spdx"` and `"cyclonedx"`. |
| `producer_name` | `"Eclipse Foundation"` | Organisation name recorded as the SBOM producer. |
| `producer_url` | Eclipse S-CORE URL | URL of the SBOM producer organisation. |
| `sbom_authors` | `None` | List of author strings written into SBOM metadata; defaults to `producer_name` when omitted. |
| `namespace` | `https://eclipse.dev/score` | URI used as the SPDX document namespace and CycloneDX serial number base. |
| `generation_context` | `None` | CycloneDX lifecycle phase label (e.g. `"build"`, `"release"`). |
| `sbom_tools` | `None` | List of tool name strings recorded in SBOM metadata alongside the generator itself. |
| `exclude_patterns` | `None` | List of repo name substrings to exclude from the dependency graph (e.g. build tools, test frameworks). |
| `dep_module_files` | `None` | `MODULE.bazel` files from dependency modules used for additional automatic version extraction. |
| `metadata_json` | `@sbom_metadata//:metadata.json` | Label to the metadata JSON produced by the `sbom_metadata` Bazel extension; rarely needs changing. |

## 3. Install Prerequisites

**Rust crate metadata** (`auto_crates_cache = True`):

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
sudo apt install openjdk-11-jre-headless  # or equivalent for your distro
```

**C++ dependency scanning** (`auto_cdxgen = True`):

```bash
nvm install 20
npm install -g @cyclonedx/cdxgen
```

Set `auto_cdxgen = False` if cdxgen is not available.

## 4. Build

```bash
bazel build //:my_sbom
```

## 5. Output

Generated in `bazel-bin/`:

- `my_sbom.spdx.json` — SPDX 2.3
- `my_sbom.cdx.json` — CycloneDX 1.6
- `my_sbom_crates_metadata.json` — Rust crate cache (if `auto_crates_cache = True`)
- `my_sbom_cdxgen.cdx.json` — C++ scan output (if `auto_cdxgen = True`)

---

## Architecture

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
- **dash-license-scan** — licenses data
- **crates.io API** — description and supplier for Rust crates
- **cdxgen** — C++ dependency licenses, descriptions, and suppliers

### Automated Metadata Sources

All license, hash, supplier, and description values are derived from automated sources: `MODULE.bazel.lock`, `http_archive` rules, dash-license-scan (Rust), crates.io API (Rust), and cdxgen (C++). Cache files such as `cpp_metadata.json` must never be hand-edited.

CPE, aliases, and pedigree are the only fields that may be set manually via `sbom_ext.license()`, as they represent identity and provenance annotations that cannot be auto-deduced.

### Required SBOM Fields (CISA 2025)

Every component entry in the generated SBOM must include the following fields, as mandated by CISA 2025 minimum elements:

| Field | SPDX 2.3 | CycloneDX 1.6 | Source | Description |
|---|---|---|---|---|
| Component name | `name` | `components[].name` | Extracted | Human-readable name of the dependency (e.g. `serde`, `boost.mp11`). |
| Component version | `versionInfo` | `components[].version` | Extracted | Exact released version string used in the build. |
| Component hash (SHA-256) | `checksums[SHA256]` | `components[].hashes` | Extracted | SHA-256 digest of the downloaded archive, sourced from `MODULE.bazel.lock` or the `http_archive` `sha256` field. |
| Software identifier (PURL) | `externalRefs[purl]` | `components[].purl` | Extracted | Package URL uniquely identifying the component by ecosystem, name, and version (e.g. `pkg:cargo/serde@1.0.228`). |
| License expression | `licenseConcluded` | `components[].licenses` | Extracted | SPDX license expression concluded for this component (e.g. `Apache-2.0 OR MIT`). |
| Dependency relationships | `relationships[DEPENDS_ON]` | `dependencies` | Extracted | Graph edges recording which component depends on which, enabling consumers to reason about transitive exposure. |
| Supplier | `supplier` | `components[].supplier.name` | Extracted | Organisation or individual that distributes the component (e.g. the crates.io publisher name). |
| Component description | `description` | `components[].description` | Extracted | Short human-readable summary of what the component does; set to `"Missing"` when no source can provide it. |
| SBOM author | `creationInfo.creators` | `metadata.authors` | Configured | Entity responsible for producing this SBOM document; set via `producer_name` in the `sbom()` rule (default: Eclipse Foundation). |
| Tool name | `creationInfo.creators` | `metadata.tools` | Auto-generated | Name and version of the tool that generated the SBOM. |
| Timestamp | `creationInfo.created` | `metadata.timestamp` | Auto-generated | ISO-8601 UTC timestamp recording when the SBOM was generated. |
| Generation context (lifecycle) | — | `metadata.lifecycles` | Auto-generated | CycloneDX lifecycle phase at which the SBOM was produced (e.g. `build`). |

Legend: **Extracted** — derived automatically from the Bazel dependency graph, lockfiles, or external registries (crates.io, cdxgen). **Configured** — comes from an `sbom()` rule parameter with a sensible default. **Auto-generated** — computed at build time with no user input required.

Fields are populated automatically from the sources described in [Automated Metadata Sources](#automated-metadata-sources) and [License Data by Language](#license-data-by-language). If a source cannot provide a value (e.g. cdxgen cannot resolve a C++ component), the field is omitted rather than filled with incorrect data — except for description, which is set to `"Missing"` to make the gap visible.

### Component Scope

Only transitive dependencies of the declared build targets are included. Build-time tools (compilers, build systems, test frameworks) are excluded via `exclude_patterns`.

### Component Hash Source

SHA-256 checksums come exclusively from `MODULE.bazel.lock` `registryFileHashes` (BCR modules) or the `sha256` field of `http_archive` rules. If neither provides a checksum, the hash field is omitted rather than emitting an incorrect value.

### License Data by Language

- **Rust**: Licenses via dash-license-scan (Eclipse Foundation + ClearlyDefined); descriptions and suppliers from crates.io API. Crates with platform-specific suffixes (e.g. `iceoryx2-bb-lock-free-qnx8`) fall back to the base crate name for lookup.
- **C++**: Licenses, descriptions, and suppliers via cdxgen source tree scan. There is no dash-license-scan integration for C++ — it does not support `pkg:generic/...` PURLs used by BCR modules. If cdxgen cannot resolve a component, its description is set to `"Missing"` and its license field is empty.

### Output Format Versions

- **SPDX 2.3**: Migration to SPDX 3.0 is deferred until supported in production by at least one major consumer (Trivy, GitHub Dependabot, or Grype). As of early 2026, none support it and the reference Python library marks its own 3.0 support as experimental. `LicenseRef-*` identifiers are declared in `hasExtractedLicensingInfos` as required by SPDX 2.3; supplier is emitted as `Organization: <name>`.
- **CycloneDX 1.6**: Emitted with `"specVersion": "1.6"` and `"$schema": "http://cyclonedx.org/schema/bom-1.6.schema.json"`.


## How design is tested

To run tests
```bash
# From tooling/ — run all SBOM tests
bazel test //sbom/tests/...
```

Sbom was also tested by external tool
https://sbomgenerator.com/tools/validator

#### Tests description

| Test file | Bazel target | What it covers |
|---|---|---|
| `test_bcr_known_licenses.py` | `test_bcr_known_licenses` | `BCR_KNOWN_LICENSES` table; `apply_known_licenses()` priority chain (5 levels); `resolve_component()` integration after license resolution |
| `test_cpp_enrich_checksum.py` | `test_cpp_enrich_checksum` | `enrich_components_from_cpp_cache()` field propagation (checksum, normalised names, parent match); no-manual-curation rule on `cpp_metadata.json` |
| `test_cyclonedx_formatter.py` | `test_cyclonedx_formatter` | CycloneDX 1.6 document structure; license encoding (single id vs compound expression); `or`/`and` normalisation; dependency graph; `_normalize_spdx_license()` |
| `test_spdx_formatter.py` | `test_spdx_formatter` | SPDX 2.3 document structure; PURL as externalRef; SHA-256 checksums; DESCRIBES/DEPENDS_ON relationships; `hasExtractedLicensingInfos` for `LicenseRef-*`; `_normalize_spdx_license()` |
| `test_sbom_generator.py` | `test_sbom_generator` | `filter_repos()`; `resolve_component()` (all 8 repo-type branches); `deduplicate_components()`; `parse_module_bazel_files()`; `parse_module_lockfiles()`; `mark_missing_cpp_descriptions()`; `main()` end-to-end (15 scenarios: SPDX/CycloneDX output, BCR licenses, crate_universe, exclude patterns, version auto-detect, dep_module_files, module_lockfiles, --crates-cache, --cdxgen-sbom, output file selection) |
| `test_generate_crates_metadata_cache.py` | `test_generate_crates_metadata_cache` | `parse_dash_summary()`; `parse_module_bazel_lock()`; `generate_synthetic_cargo_lock()`; end-to-end summary CSV round-trip |
| `test_generate_cpp_metadata_cache.py` | `test_generate_cpp_metadata_cache` | `convert_cdxgen_to_cache()`: version, license (id/name/expression/AND), supplier (name/publisher fallback), PURL, URL from externalReferences, description |
| `test_spdx_to_github_snapshot.py` | `test_spdx_to_github_snapshot` | `convert_spdx_to_snapshot()`: top-level fields; direct vs. indirect classification; package filtering; manifest naming; `pkg:generic/` PURL support |

---
