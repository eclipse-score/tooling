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

#include <string>
#include <unordered_map>

class MyUnit {
public:
  // trace: MinimalExample.FEAT_001
  void configure(const std::string &key, const std::string &value);

  // trace: MinimalExample.FEAT_002
  std::string get(const std::string &key) const;

private:
  std::unordered_map<std::string, std::string> store_;
};
