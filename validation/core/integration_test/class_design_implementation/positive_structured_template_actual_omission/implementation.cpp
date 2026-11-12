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

namespace structured {

template <typename T>
struct Inner {};

template <typename T, typename Value, typename Policy = double>
struct Envelope {};

struct Consumer {
    using Payload = Envelope<Inner<int>, float>;

    Payload value;
    const Payload* wrapped;
    const Payload& Read(const Payload& input);
    void Normalize(Payload value, Payload* ptr);
    void Cv(const volatile Payload* const volatile* input);
};

}  // namespace structured
