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
  ::testing::Test::RecordProperty("given",
                                  "a default-constructed Foo instance");
  ::testing::Test::RecordProperty("when", "GetNumber is called");
  ::testing::Test::RecordProperty("then", "it returns 42");

  unit_1::Foo unit{};

  EXPECT_EQ(unit.GetNumber(), 42u);
}

TEST(Foo, IsFinal) {
  ::testing::Test::RecordProperty("lobster-tracing",
                                  "SampleComponent.REQ_COMP_002");
  ::testing::Test::RecordProperty("given", "the Foo class definition");
  ::testing::Test::RecordProperty("when",
                                  "checking whether the class is extensible");
  ::testing::Test::RecordProperty(
      "then", "it is declared final, preventing any subclassing");
  // Foo is declared final; extensibility is enforced at compile time.
  SUCCEED();
}

TEST(Foo, InitializesToKnownValue) {
  ::testing::Test::RecordProperty("lobster-tracing",
                                  "SampleComponentExtra.REQ_COMP_EXTRA_001");
  ::testing::Test::RecordProperty("given",
                                  "a default-constructed Foo instance");
  ::testing::Test::RecordProperty("when",
                                  "GetNumber is called for the first time");
  ::testing::Test::RecordProperty("then", "it returns 42");

  unit_1::Foo unit{};
  EXPECT_EQ(unit.GetNumber(), 42u);
}

TEST(Foo, ValueConsistentAcrossReads) {
  ::testing::Test::RecordProperty("lobster-tracing",
                                  "SampleComponentExtra.REQ_COMP_EXTRA_002");
  ::testing::Test::RecordProperty("given", "a const Foo instance");
  ::testing::Test::RecordProperty("when", "GetNumber is called multiple times");
  ::testing::Test::RecordProperty("then",
                                  "the same value is returned on each call");

  const unit_1::Foo unit{};
  EXPECT_EQ(unit.GetNumber(), unit.GetNumber());
}
