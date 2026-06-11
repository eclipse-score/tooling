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
// Concrete classes that depend on transport.h
#include "transport.h"

namespace MyNamespace
{
namespace MyNamespace2
{
// Concrete class: implements all inherited pure virtual methods.
class Car : public VehicleBase
{
  public:
    Car(const char* name, int doors);

    void start() override;
    void stop() override;
    int getDoors() const;
    int wheelCount() const override;

  private:
    int m_doors;
};
}  // namespace MyNamespace2
}  // namespace MyNamespace
