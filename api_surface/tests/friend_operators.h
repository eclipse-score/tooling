// *******************************************************************************
// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
// SPDX-License-Identifier: Apache-2.0
// *******************************************************************************

#pragma once

namespace test
{

class Printer;

/// \brief A value type with friend functions and non-member operators.
/// \api
class Value
{
  public:
    Value();

    /// \brief Hidden friend, defined in-class.
    /// \api
    friend bool operator==(const Value& lhs, const Value& rhs) noexcept
    {
        return &lhs == &rhs;
    }

    /// \brief Hidden friend with an attribute.
    /// \api
    [[nodiscard]] friend bool operator<(const Value& lhs, const Value& rhs)
    {
        return &lhs < &rhs;
    }

    /// \brief Friend declared in-class and defined elsewhere.
    /// \api
    friend Value operator+(const Value& lhs, const Value& rhs);

    /// \brief Friend function template.
    /// \api
    template <typename T>
    friend T Convert(const Value& value);

    // A friend class is not a function and is not part of the API surface.
    friend class Printer;

  private:
    /// \brief Friend declared in the private section is still public API.
    /// \api
    friend void swap(Value& lhs, Value& rhs) noexcept;

    int state_{};
};

/// \brief Non-member operator.
/// \api
bool operator!=(const Value& lhs, const Value& rhs) noexcept;

/// \brief Non-member operator template.
/// \api
template <typename T>
bool operator>(const T& lhs, const Value& rhs);

// Friend without documentation.
class Undocumented
{
  public:
    friend bool operator==(const Undocumented&, const Undocumented&)
    {
        return true;
    }
};

class Outer
{
  public:
    class Inner
    {
      public:
        friend bool operator==(const Inner&, const Inner&)
        {
            return true;
        }
    };
};

namespace detail
{

class Hidden
{
  public:
    friend bool operator==(const Hidden&, const Hidden&)
    {
        return true;
    }
};

}  // namespace detail

}  // namespace test
