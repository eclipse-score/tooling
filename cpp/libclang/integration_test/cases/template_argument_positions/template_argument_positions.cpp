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

namespace argument_positions
{

template <typename First, int Count, typename Last>
class Mixed
{
};

class Probe
{
  public:
    // The non-type middle argument must not disappear or shift float left.
    Mixed<int, 3, float> value;
    void accept(Mixed<int, 3, float> input);
    Mixed<int, 3, float> plain();

    // Keep outer const/volatile exactly once; preserve qualifiers inside template arguments.
    const Mixed<int, 3, float> constant();
    volatile Mixed<int, 3, float> varying();
    const volatile Mixed<int, 3, float> qualified();
    const Mixed<const int, 3, volatile float> inner_qualified();
};

template <bool Flag, unsigned long long Maximum, int* Pointer, typename Last>
class ValueKinds
{
  public:
    using Example = ValueKinds<true, 18446744073709551615ULL, nullptr, int>;
    using Negative = Mixed<int, -3, float>;
};

}  // namespace argument_positions
