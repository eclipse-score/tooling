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
namespace Rel {
class IService {
  public:
    virtual ~IService() = default;
    virtual void run() = 0;
};
} // namespace Rel

namespace Vehicle {
  class Car : public Rel::IService {
public:
    void run() override;
};
} // namespace Vehicle
