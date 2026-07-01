/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

#include <cstdint>

namespace vehicle
{
struct Manufacturer
{
    int id;
};

enum class Mode
{
    Off = 0,
    On = 1,
};

template <typename T>
class Box
{
};

struct Engine
{
    using Speed = std::uint8_t;

    int speed;
    static int instance_count;
    Manufacturer vendor;

    void start(int mode);
    void configure(int mode, int force);
    Speed current_speed();
    static void Reset();

  private:
    // NOTE: this is not part of the puml file but the test still passes
    // since we want to be able to ommit implementation details
    int status;
    void calibrate();
    int implementation_detail;
};
}  // namespace vehicle
