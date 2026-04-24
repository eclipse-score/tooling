// *******************************************************************************
// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
// SPDX-License-Identifier: Apache-2.0
// *******************************************************************************

// Unit tests for test_binary_unit fixture
#include <gtest/gtest.h>

// Declarations from mock libraries
extern int mock_function_1();
extern int mock_function_2();

TEST(BinaryUnitTest, MockFunction1ReturnsExpectedValue) {
    ::testing::Test::RecordProperty("lobster-tracing", "TestComponent.REQ_COMP_TEST_001");
    EXPECT_EQ(mock_function_1(), 42);
}

TEST(BinaryUnitTest, MockFunction2ReturnsExpectedValue) {
    ::testing::Test::RecordProperty("lobster-tracing", "TestComponent.REQ_COMP_TEST_001");
    EXPECT_EQ(mock_function_2(), 84);
}
