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

#ifndef CPP_LIBCLANG_INTEGRATION_TEST_CASES_TEMPLATE_PARTIAL_SPECIALIZATION_IDS_SPECIALIZATION_WITH_RETURN_H
#define CPP_LIBCLANG_INTEGRATION_TEST_CASES_TEMPLATE_PARTIAL_SPECIALIZATION_IDS_SPECIALIZATION_WITH_RETURN_H

#include "specialization.h"

namespace demo
{

// Covers a partial specialization that binds both a regular template parameter
// and a parameter pack from a function-signature template argument.
template <typename ReturnType, typename... ArgTypes>
class Endpoint<ReturnType(ArgTypes...)>
{
  public:
    ReturnType Request(const ArgTypes&... args);

  private:
    int parameter_count_;
};

}  // namespace demo

#endif  // CPP_LIBCLANG_INTEGRATION_TEST_CASES_TEMPLATE_PARTIAL_SPECIALIZATION_IDS_SPECIALIZATION_WITH_RETURN_H
