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
#pragma once

namespace MyNamespace
{
namespace MyNamespace2
{
// Interface: only pure virtual methods, no data members.
class ITransport
{
  public:
    virtual ~ITransport() = default;
    virtual void start() = 0;
    virtual void stop() = 0;
};

// Abstract class: has data members and at least one pure virtual method.
class VehicleBase : public ITransport
{
  public:
    explicit VehicleBase(const char* name);
    virtual ~VehicleBase() = default;

    // Const methods
    virtual int wheelCount() const = 0;

    // Static methods
    static int instanceCount();
    static VehicleBase* create(const char* name);

  protected:
    int m_speed;

  private:
    const char* m_name;
    char* const m_id;
    const int m_max_speed;
    const int* const m_firmware;
    const int& m_ref;
};
}  // namespace MyNamespace2
}  // namespace MyNamespace
