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

#include <gtest/gtest.h>

TEST(Foo, GetNumber) {
  ::testing::Test::RecordProperty("lobster-tracing",
                                  "SampleComponent.REQ_COMP_001");

  unit_1::Foo unit{};

  EXPECT_EQ(unit.GetNumber(), 42u);
}
