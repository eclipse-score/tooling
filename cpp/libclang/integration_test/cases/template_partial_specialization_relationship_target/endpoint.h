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

#ifndef CPP_LIBCLANG_INTEGRATION_TEST_CASES_TEMPLATE_PARTIAL_SPECIALIZATION_RELATIONSHIP_TARGET_ENDPOINT_H
#define CPP_LIBCLANG_INTEGRATION_TEST_CASES_TEMPLATE_PARTIAL_SPECIALIZATION_RELATIONSHIP_TARGET_ENDPOINT_H

namespace demo
{

// Primary template exists as a distinct entity, and the relationship logic
// currently prefers this template base over a concrete specialization target.
template <typename Signature>
class Endpoint
{
    static_assert(sizeof(Signature) == 0, "Endpoint only supports function-signature template arguments.");
};

}  // namespace demo

#endif  // CPP_LIBCLANG_INTEGRATION_TEST_CASES_TEMPLATE_PARTIAL_SPECIALIZATION_RELATIONSHIP_TARGET_ENDPOINT_H
