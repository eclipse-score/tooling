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

#include <cstddef>

namespace amp
{
template <class Signature, std::size_t Capacity, std::size_t Alignment>
class move_only_function;

template <typename ReturnType, typename... ArgTypes, std::size_t Capacity, std::size_t Alignment>
class move_only_function<ReturnType(ArgTypes...), Capacity, Alignment>
{
public:
    ReturnType operator()(ArgTypes... args) const;
    bool empty() const noexcept;

private:
    class wrapper_base
    {
    public:
        virtual ~wrapper_base() = default;
        virtual ReturnType invoke(ArgTypes... args) const = 0;
        virtual wrapper_base* relocate(void* destination) = 0;
    };

    template <typename Callable>
    class wrapper final : public wrapper_base
    {
    public:
        ReturnType invoke(ArgTypes... args) const final;
        wrapper_base* relocate(void* destination) final;
    };
};

template <typename ReturnType, typename... ArgTypes, std::size_t Capacity, std::size_t Alignment>
class move_only_function<ReturnType (*)(ArgTypes...), Capacity, Alignment>
{
public:
    bool empty() const noexcept;
};

// Full specializations share the same design contract as partial specializations.
template <>
class move_only_function<void(), 32, 8>
{
public:
    bool empty() const noexcept;
};

template <>
class move_only_function<int(), 64, 8>
{
public:
    bool empty() const noexcept;
};
}  // namespace amp
