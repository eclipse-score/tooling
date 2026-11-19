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

#include <functional>
#include <memory>
#include <utility>

namespace storage {

struct ElementFqId {};
struct EventMetaInfo {};

template <typename Key,
          typename MappedType,
          typename KeyEqual = std::equal_to<Key>,
          typename Allocator = std::allocator<std::pair<Key, MappedType>>>
struct LinearSearchMap {};

class ServiceDataStorage {
  public:
    using EventMetaInfoMap = LinearSearchMap<ElementFqId, EventMetaInfo>;

  private:
    // Written with two actuals; the parser expands the two default arguments.
    EventMetaInfoMap events_metainfo_;
};

}  // namespace storage
