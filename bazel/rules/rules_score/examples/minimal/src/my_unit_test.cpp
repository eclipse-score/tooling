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

#include <gtest/gtest.h>

#include "src/my_unit.h"

TEST(MyUnitTest, ConfigureAndGet) {
  ::testing::Test::RecordProperty(
      "lobster-tracing", "MinimalExample.FEAT_001 MinimalExample.FEAT_002");
  ::testing::Test::RecordProperty("given",
                                  "a default-constructed MyUnit instance");
  ::testing::Test::RecordProperty("when",
                                  "configure is called with a known key");
  ::testing::Test::RecordProperty("then", "get returns the configured value");

  // Given a default-constructed MyUnit instance
  MyUnit unit{};

  // When configure is called with a key that hasn't been configured yet
  unit.configure("mode", "fast");

  // Then get returns the configured value
  EXPECT_EQ(unit.get("mode"), "fast");
}

TEST(MyUnitTest, MissingKeyReturnsEmpty) {
  ::testing::Test::RecordProperty("lobster-tracing", "MinimalExample.FEAT_002");

  // Given a default-constructed MyUnit instance
  MyUnit unit{};

  // When get is called with a key that hasn't been configured yet
  const auto retrieved_value = unit.get("undefined");

  // Then get returns an empty string
  EXPECT_EQ(retrieved_value, "");
}
