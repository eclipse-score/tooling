/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

struct Engine
{
    Engine();
    ~Engine();

    template <typename Payload>
    bool build();

    // bool run(int mode, int payload, ...);

    int select(int mode);
    int select(int mode, int force);

    static Engine* Create(const char* name);

  private:
    void reset(int reason);
};
