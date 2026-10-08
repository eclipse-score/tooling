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

#include "unit_3/formatter.h"

#include <gtest/gtest.h>

namespace unit_1 = safety_software_seooc_example::component_example::unit_1;
namespace unit_3 = safety_software_seooc_example::component_example::sub_component_example::unit_3;

TEST(Formatter, Format) {
  ::testing::Test::RecordProperty("lobster-tracing",
                                  "SampleComponent.REQ_COMP_006");

  ::testing::Test::RecordProperty("given",
                                  "a Formatter and a default-constructed Foo");
  unit_3::Formatter formatter;
  unit_1::Foo foo;

  ::testing::Test::RecordProperty("when", "Format is called with the Foo");
  ::testing::Test::RecordProperty(
      "then", "it returns the human-readable string \"42\"");
  EXPECT_EQ(formatter.Format(foo), "42");
}
