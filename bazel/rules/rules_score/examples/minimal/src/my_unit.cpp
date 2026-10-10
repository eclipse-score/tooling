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

#include "my_unit.h"

namespace my_element {
namespace MyComponent {
namespace MyUnit {

void KeyValueStore::configure(const std::string &key,
                             const std::string &value) {
  store_[key] = value;
}

std::string KeyValueStore::get(const std::string &key) const {
  const auto it = store_.find(key);
  return it != store_.end() ? it->second : "";
}

} // namespace MyUnit
} // namespace MyComponent
} // namespace my_element
