/****************************************************************************
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
 ****************************************************************************/

// Compared: the design declares Box<T> with value and get().
template <typename T>
class Box
{
public:
    T value;

    // Collected, but not compared: Box::Nested is not declared in this case's design.
    class Nested {};

    T get() const
    {
        return value;
    }
};

// For Box, only the primary template Box<T> above is compared against the design.
// Explicit instantiation definition: reuses Box<T>; excluded from comparison.
template class Box<double>;
// Explicit instantiation declaration: reuses Box<T>; excluded from comparison.
extern template class Box<float>;

// Implicit instantiation: Box<char> is not collected as a separate class for comparison.
Box<char> implicit_box{};

// Not compared: this primary template is only a forward declaration, not a definition.
template <typename T>
class Contract;

// Compared: this partial specialization must provide the design's bool empty().
template <typename T>
class Contract<T*>
{
public:
    bool empty() const;
};

// Compared: this full specialization must also provide the design's bool empty().
// The comment in the template header must not affect specialization detection.
template /* Comments between template-header tokens are ignored. */ < >
class Contract<int>
{
public:
    bool empty() const;
};

// Not compared: a full-specialization forward declaration has no implementation.
template <>
class Contract<long>;

// Compared: the design declares this ordinary class.
class Plain {};
