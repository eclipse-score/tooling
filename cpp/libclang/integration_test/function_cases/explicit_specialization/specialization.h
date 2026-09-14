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

#pragma once

namespace utility {

// Declaration-only primary template:
// The test targets the explicit specialization below.
template <typename T>
T specialized(T value);

// Explicit specialization should still be extracted as a concrete function definition.
template <>
int specialized(int value) {
    return value;
}

}  // namespace utility
