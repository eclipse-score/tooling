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

#ifndef CPP_LIBCLANG_INTEGRATION_TEST_CASES_TEMPLATE_PARTIAL_SPECIALIZATION_IDS_SPECIALIZATION_H
#define CPP_LIBCLANG_INTEGRATION_TEST_CASES_TEMPLATE_PARTIAL_SPECIALIZATION_IDS_SPECIALIZATION_H

namespace demo
{

// Primary template kept in its own header so the parser has to merge it with
// function-signature partial specializations declared elsewhere.
template <typename Signature>
class Endpoint
{
	static_assert(sizeof(Signature) == 0, "Endpoint only supports function-signature template arguments.");
};

}  // namespace demo

#endif  // CPP_LIBCLANG_INTEGRATION_TEST_CASES_TEMPLATE_PARTIAL_SPECIALIZATION_IDS_SPECIALIZATION_H
