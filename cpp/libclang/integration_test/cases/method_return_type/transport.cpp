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

#include <map>
#include <vector>

namespace external_types {
struct RouteInfo {
    int route_id;
};
}  // namespace external_types

using namespace external_types;

class MethodReturnTypeSample {
  public:
    const char* getName();
    const int& getIdRef() const;
    const int&& getIdRValueRef() const;
    int* getMutableIdPtr();
    const int* const getPinnedIdPtr() const;

    std::vector<int> getHistory() const;
    std::vector<const char*> getNames() const;

    std::vector<std::vector<const int*>> getConstPointerMatrix() const;
    std::map<int, std::vector<const char*>> getNameIndex() const;

    std::map<int, RouteInfo> getRouteInfoById() const;
};
