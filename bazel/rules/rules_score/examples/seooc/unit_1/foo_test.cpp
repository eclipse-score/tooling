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

#include "unit_1/foo.h"

#include <gtest/gtest.h>

#include <type_traits>

TEST(Foo, GetNumber) {
  ::testing::Test::RecordProperty("lobster-tracing",
                                  "SampleComponent.REQ_COMP_001");

  ::testing::Test::RecordProperty("given",
                                  "a default-constructed Foo instance");
  unit_1::Foo unit{};

  ::testing::Test::RecordProperty("when", "GetNumber is called");
  ::testing::Test::RecordProperty("then", "it returns 42");
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

TEST(Foo, IsFinalStaticAssert) {
  ::testing::Test::RecordProperty("lobster-tracing",
                                  "SampleComponent.REQ_COMP_002");

  ::testing::Test::RecordProperty("given", "the Foo class definition");
  ::testing::Test::RecordProperty(
      "when", "querying the type trait std::is_final for Foo");
  ::testing::Test::RecordProperty(
      "then", "the trait reports true, confirming Foo cannot be subclassed");
  static_assert(std::is_final<unit_1::Foo>::value, "Foo must remain final");
  SUCCEED();
}

TEST(Foo, GetNumberViaConstInstance) {
  ::testing::Test::RecordProperty("lobster-tracing",
                                  "SampleComponent.REQ_COMP_001");

  ::testing::Test::RecordProperty("given",
                                  "a const default-constructed Foo instance");
  const unit_1::Foo unit{};

  ::testing::Test::RecordProperty(
      "when", "GetNumber is called through a const reference");
  ::testing::Test::RecordProperty("then", "it still returns 42");
  EXPECT_EQ(unit.GetNumber(), 42u);
}

TEST(Foo, InitializesToKnownValue) {
  ::testing::Test::RecordProperty("lobster-tracing",
                                  "SampleComponentExtra.REQ_COMP_EXTRA_001");

  ::testing::Test::RecordProperty("given",
                                  "a default-constructed Foo instance");
  unit_1::Foo unit{};

  ::testing::Test::RecordProperty("when",
                                  "GetNumber is called for the first time");
  ::testing::Test::RecordProperty("then", "it returns 42");
  EXPECT_EQ(unit.GetNumber(), 42u);
}

TEST(Foo, ValueConsistentAcrossReads) {
  ::testing::Test::RecordProperty("lobster-tracing",
                                  "SampleComponentExtra.REQ_COMP_EXTRA_002");

  ::testing::Test::RecordProperty("given", "a const Foo instance");
  const unit_1::Foo unit{};

  ::testing::Test::RecordProperty("when", "GetNumber is called multiple times");
  ::testing::Test::RecordProperty("then",
                                  "the same value is returned on each call");
  EXPECT_EQ(unit.GetNumber(), unit.GetNumber());
}
