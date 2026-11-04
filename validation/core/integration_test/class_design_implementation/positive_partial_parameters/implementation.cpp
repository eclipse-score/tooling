/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

namespace partial {
namespace types {
struct Key {};
struct Value {};
struct Compare {};
struct Allocator {};
}  // namespace types

namespace containers {
// Every use must spell all four actual arguments; there are no defaults.
template <typename K, typename V, typename C, typename A>
struct Map {};
}  // namespace containers

// Declaration parameter lists, including nested template-template lists, may
// omit trailing entries in the design. None of these declarations has defaults.
template <typename T, int N>
class Declaration {};

template <template <typename, int> class C, typename Policy>
class NestedDeclaration {};

struct Consumer {
    using Storage = partial::containers::Map<partial::types::Key, partial::types::Value,
                                             partial::types::Compare, partial::types::Allocator>;
    partial::containers::Map<partial::types::Key, partial::types::Value,
                            partial::types::Compare, partial::types::Allocator> values;

    const partial::containers::Map<partial::types::Key, partial::types::Value,
                                   partial::types::Compare, partial::types::Allocator>&
    Read(int mode, int attempts) const {
        return values;
    }

    void Write(const partial::containers::Map<partial::types::Key, partial::types::Value,
                                             partial::types::Compare, partial::types::Allocator>& value,
               int attempts) {}

    template <typename T, int N>
    int Convert(int mode, int attempts) { return mode + attempts; }

    int Select(int mode) { return mode; }
    int Select(int mode, int force) { return mode + force; }

    // Empty design parameters must select only the zero-parameter overload.
    int Zero() { return 0; }
    int Zero(int mode) { return mode; }
};

partial::containers::Map<partial::types::Key, partial::types::Value,
                         partial::types::Compare, partial::types::Allocator>
Make(int mode, int attempts) {
    return {};
}

void Submit(const partial::containers::Map<partial::types::Key, partial::types::Value,
                                          partial::types::Compare, partial::types::Allocator>& value,
            bool force) {}

template <typename T, int N>
int Convert(int mode, int attempts) { return mode + attempts; }

int Select(int mode) { return mode; }
int Select(int mode, int force) { return mode + force; }
int Zero() { return 0; }
int Zero(int mode) { return mode; }
}  // namespace partial
