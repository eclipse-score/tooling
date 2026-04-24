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

// Main implementation for test_component
#include <iostream>

// Declarations from mock libraries
extern int mock_function_1();
extern int mock_function_2();

int main(int argc, char** argv) {
    std::cout << "Test Component Implementation" << std::endl;
    std::cout << "Mock function 1 returns: " << mock_function_1() << std::endl;
    std::cout << "Mock function 2 returns: " << mock_function_2() << std::endl;
    return 0;
}
