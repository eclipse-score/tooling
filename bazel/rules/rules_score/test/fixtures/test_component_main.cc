// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// SPDX-License-Identifier: Apache-2.0

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
