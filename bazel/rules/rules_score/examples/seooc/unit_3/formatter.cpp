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

#include "unit_3/formatter.h"

namespace unit_3 {

// trace: SampleComponent.REQ_COMP_006
std::string Formatter::Format(const unit_1::Foo &foo) const {
  return std::to_string(foo.GetNumber());
}

} // namespace unit_3
