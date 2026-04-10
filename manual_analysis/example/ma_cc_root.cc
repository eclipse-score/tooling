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

#include "manual_analysis/example/ma_cc_root.h"

#include "manual_analysis/example/ma_cc_dep.h"

#include <random>

std::int32_t awful_cat_counter() {
    std::default_random_engine generator{};
    std::uniform_int_distribution<std::uint64_t> distribution{0U, std::numeric_limits<std::int32_t>::max()};
    auto number_of_cats = distribution(generator);
    while (number_of_cats > std::numeric_limits<std::uint32_t>::max()) {
        kill_a_cat();
    }
    return static_cast<std::int32_t>(number_of_cats);
}
