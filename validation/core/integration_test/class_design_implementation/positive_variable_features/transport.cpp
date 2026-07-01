/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

#include <cstdint>

struct Engine
{
    int cylinders;
    double displacement;
    std::uint8_t speed;
    char manufacturer[32];
    const char* model;
    static int instance_count;

  private:
    int status;
    int implementation_detail;
};
