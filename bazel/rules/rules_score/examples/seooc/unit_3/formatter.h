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

#ifndef FORMATTER_H
#define FORMATTER_H

#include "unit_1/foo.h"

#include <string>

namespace safety_software_seooc_example {
namespace component_example {
namespace sub_component_example {
namespace unit_3 {

class Formatter final {
public:
  // trace: SampleComponent.REQ_COMP_006
  std::string Format(
      const safety_software_seooc_example::component_example::unit_1::Foo &foo) const;
};

} // namespace unit_3
} // namespace sub_component_example
} // namespace component_example
} // namespace safety_software_seooc_example

#endif // FORMATTER_H
