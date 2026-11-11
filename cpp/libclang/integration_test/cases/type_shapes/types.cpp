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

template <typename First, typename Second> struct Pair {};
template <typename Value, int Count> struct Mixed {};

struct Widget {
    // Alias/member metadata ignores outer cv/pointer and preserves inner qualifiers.
    using TemplateAlias = const Pair<int *const, volatile float *> *;
    const Pair<int *const, volatile float *> *wrapped;

    // A nested template is preserved as one complete argument.
    Pair<int, Pair<float, const int *>> nested;

    // Mixed non-type/function arguments fall back to the original type spelling.
    Mixed<const int *, 3> mixed;
    Pair<int, void (*)(int)> unsupported_nested;

    // Extract template metadata from method return and parameter types.
    const Pair<int *const, volatile float *> *template_method(
        const Pair<int *const, volatile float *> &value);
};

// Extract template metadata from free-function return and parameter types.
const Pair<int *const, volatile float *> *free_template(
    const Pair<int *const, volatile float *> &value);
