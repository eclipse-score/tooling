/********************************************************************************
 * Copyright (c) 2025 Contributors to the Eclipse Foundation
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

#include "bazel/rules/rules_score/examples/seooc/unit_1/foo.h"

namespace unit_1 {

// trace: SampleComponent.REQ_COMP_001 SampleLibraryAPI.GetNumber
std::uint8_t Foo::GetNumber() const { return 42u; }
} // namespace unit_1
