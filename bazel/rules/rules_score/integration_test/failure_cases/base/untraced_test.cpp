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

#include <gtest/gtest.h>

#include "my_unit.h"

TEST(MyUnit, Returns42WithoutTrace) {
    ::testing::Test::RecordProperty("given", "a MyUnit instance");
    MyUnit unit{};

    ::testing::Test::RecordProperty("when", "Value is called");
    const auto value = unit.Value();

    ::testing::Test::RecordProperty("then", "it returns 42");
    EXPECT_EQ(value, 42U);
}
