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

// Regression test: a workspace class that inherits from an external-library
// base class must not cause the AST parser to panic.  The base class is
// filtered out during the visit phase (path contains "external/"), so it is
// not present in the type map.  The expected result is that the workspace
// class is emitted without any inheritance relationship.
#include "flatbuffers/flatbuffers.h"

namespace my_ns {

class MyNativeObject : public flatbuffers::NativeTable {
public:
  int value{};
};

} // namespace my_ns
