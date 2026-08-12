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
#ifndef CPP_LIBCLANG_INTEGRATION_TEST_CASES_STD_TEMPLATE_SPECIALIZATION_FILTER_SPECIALIZATION_H
#define CPP_LIBCLANG_INTEGRATION_TEST_CASES_STD_TEMPLATE_SPECIALIZATION_FILTER_SPECIALIZATION_H

#include <cstddef>
#include <cstdint>
#include <functional>

namespace demo
{

struct Key
{
    std::uint32_t value;
};

}  // namespace demo

namespace std
{

template <>
struct hash<demo::Key>
{
    std::size_t operator()(const demo::Key& key) const noexcept
    {
        return std::hash<std::uint32_t>{}(key.value);
    }
};

}  // namespace std

#endif
