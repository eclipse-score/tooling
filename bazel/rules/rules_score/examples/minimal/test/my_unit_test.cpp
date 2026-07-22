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

  MyUnit unit;
  unit.configure("mode", "fast");
  EXPECT_EQ(unit.get("mode"), "fast");
}

TEST(MyUnitTest, MissingKeyReturnsEmpty) {
  ::testing::Test::RecordProperty("lobster-tracing", "MinimalExample.FEAT_002");

  MyUnit unit;
  EXPECT_EQ(unit.get("undefined"), "");
}
