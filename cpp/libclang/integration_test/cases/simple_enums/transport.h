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
#ifndef TOOLS_CPP_LIBCLANG_INTEGRATION_TESTS_CASES_SIMPLE_ENUMS_TRANSPORT_H
#define TOOLS_CPP_LIBCLANG_INTEGRATION_TESTS_CASES_SIMPLE_ENUMS_TRANSPORT_H

#include <cstdint>
enum class RGBColor : int
{
    Red = -1,
    Green,
    Blue = 32767
};

using MyInt = unsigned short;
enum struct CMYKColor : MyInt
{
    Cyan = 0,
    Magenta,
    Yellow,
    Black = 65535
};

namespace MyNamespace
{
enum struct Direction : int
{
    North = 0,
    East,
    South,
    West
};

enum struct Direction3D : std::uint8_t
{
    North = 0,
    East,
    South,
    West,
    Up,
    Down = 255
};

}  // namespace MyNamespace
#endif  // TOOLS_CPP_LIBCLANG_INTEGRATION_TESTS_CASES_SIMPLE_ENUMS_TRANSPORT_H
