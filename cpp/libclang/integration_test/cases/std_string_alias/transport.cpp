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

#include <string>
#include <unordered_map>

class KeyValueStore {
public:
  std::string get(const std::string &key) const;
  void set(const std::string &key, const std::string &value);

  std::string name_;
  std::unordered_map<std::string, std::string> store_;
};
