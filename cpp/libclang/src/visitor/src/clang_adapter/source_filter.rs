// *******************************************************************************
// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// <https://www.apache.org/licenses/LICENSE-2.0>
//
// SPDX-License-Identifier: Apache-2.0
// *******************************************************************************

use clang::Type;

const SYSTEM_HEADER_PREFIXES: &[&str] = &["/usr/include", "/usr/local/include", "/opt/"];
const SYSTEM_HEADER_SUBSTRINGS: &[&str] = &["/gcc/"];
const EXTERNAL_DEP_PATH_SUBSTRINGS: &[&str] = &["/external/", "external/", "_virtual_includes/"];
const EXCLUDED_TOP_LEVEL_NAMESPACES: &[&str] = &["std", "__gnu_cxx"];

/// Returns whether a path belongs to a system header location.
pub(crate) fn is_system_header_path(path: &str) -> bool {
    SYSTEM_HEADER_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
        || SYSTEM_HEADER_SUBSTRINGS
            .iter()
            .any(|fragment| path.contains(fragment))
}

/// Returns whether a path belongs to a Bazel external dependency.
pub fn is_external_dependency_path(path: &str) -> bool {
    EXTERNAL_DEP_PATH_SUBSTRINGS
        .iter()
        .any(|fragment| path.contains(fragment))
        || (path.contains("bazel-out/") && path.contains("/external/"))
}

/// Returns whether a path belongs to a header that is outside the parsed model.
pub(crate) fn is_external_or_system_path(path: &str) -> bool {
    is_system_header_path(path) || is_external_dependency_path(path)
}

/// Returns whether a type's declaration belongs to an external or system header.
pub(crate) fn is_declared_in_external_or_system_header(ty: &Type) -> bool {
    ty.get_declaration()
        .and_then(|declaration| declaration.get_location())
        .map(|location| {
            let (path, ..) = location.get_presumed_location();
            is_external_or_system_path(&path)
        })
        .unwrap_or(false)
}

/// Returns whether an entity namespace should be excluded from the parsed model
/// even when the source file is part of the workspace.
pub(crate) fn is_excluded_namespace(namespace: Option<&str>) -> bool {
    let Some(namespace) = namespace else {
        return false;
    };

    let top_level_namespace = namespace.split("::").next().unwrap_or(namespace);
    EXCLUDED_TOP_LEVEL_NAMESPACES.contains(&top_level_namespace)
}

#[cfg(test)]
mod tests {
    use super::{
        is_excluded_namespace, is_external_dependency_path, is_external_or_system_path,
        is_system_header_path,
    };

    #[test]
    fn classifies_system_header_paths() {
        for path in [
            "/usr/include/c++/v1/vector",
            "/usr/local/include/library/header.hpp",
            "/opt/sdk/include/api.hpp",
            "/toolchains/gcc/include/c++/vector",
        ] {
            assert!(is_system_header_path(path), "expected system path: {path}");
            assert!(is_external_or_system_path(path));
        }
    }

    #[test]
    fn classifies_bazel_external_dependency_paths() {
        for path in [
            "/workspace/external/flatbuffers/include/flatbuffers.h",
            "external/flatbuffers/include/flatbuffers.h",
            "/workspace/bazel-out/k8-fastbuild/bin/_virtual_includes/runtime/flatbuffers.h",
        ] {
            assert!(
                is_external_dependency_path(path),
                "expected external dependency path: {path}"
            );
            assert!(is_external_or_system_path(path));
        }
    }

    #[test]
    fn keeps_workspace_sources_in_the_model() {
        let path = "cpp/application/include/application/car.h";
        assert!(!is_system_header_path(path));
        assert!(!is_external_dependency_path(path));
        assert!(!is_external_or_system_path(path));
    }

    #[test]
    fn excludes_standard_library_namespaces_even_for_workspace_sources() {
        assert!(is_excluded_namespace(Some("std")));
        assert!(is_excluded_namespace(Some("std::__1")));
        assert!(is_excluded_namespace(Some("__gnu_cxx")));
        assert!(is_excluded_namespace(Some("__gnu_cxx::__detail")));
    }

    #[test]
    fn keeps_project_namespaces_in_the_model() {
        assert!(!is_excluded_namespace(None));
        assert!(!is_excluded_namespace(Some("score::mw::com::impl")));
        assert!(!is_excluded_namespace(Some("amsr")));
    }
}
