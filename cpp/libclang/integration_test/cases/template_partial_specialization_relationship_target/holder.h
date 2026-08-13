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

#ifndef CPP_LIBCLANG_INTEGRATION_TEST_CASES_TEMPLATE_PARTIAL_SPECIALIZATION_RELATIONSHIP_TARGET_HOLDER_H
#define CPP_LIBCLANG_INTEGRATION_TEST_CASES_TEMPLATE_PARTIAL_SPECIALIZATION_RELATIONSHIP_TARGET_HOLDER_H

#include "endpoint_without_return.h"

namespace demo
{

// The member type names a partial specialization, but the inferred
// relationship target still resolves to the template base entity.
class Holder
{
  private:
    // relationship_target is demo::Endpoint (primary template),
    // rather than demo::Endpoint<void(int)> (specialization).
    // The relationship model currently resolves this relationship to the primary template.
    // This is a known model limitation, not an issue with the visitor pipeline.
    Endpoint<void(int)> endpoint_;
};

}  // namespace demo

#endif  // CPP_LIBCLANG_INTEGRATION_TEST_CASES_TEMPLATE_PARTIAL_SPECIALIZATION_RELATIONSHIP_TARGET_HOLDER_H
