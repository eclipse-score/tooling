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

#include <gtest/gtest.h>

extern int mock_function_1();
extern int mock_function_2();

TEST(MockLibTest, Function1Returns42AsrDirect) {
  ::testing::Test::RecordProperty("lobster-tracing",
                                  "TestComponentDirectAsr.REQ_COMP_DIRECT_ASR_001");
  EXPECT_EQ(mock_function_1(), 42);
}

TEST(MockLibTest, Function2Returns84AsrDirect) {
  ::testing::Test::RecordProperty("lobster-tracing",
                                  "TestComponentDirectAsr.REQ_COMP_DIRECT_ASR_001");
  EXPECT_EQ(mock_function_2(), 84);
}
