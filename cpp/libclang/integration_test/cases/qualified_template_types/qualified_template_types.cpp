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

namespace qualified_names
{

// Qualified hierarchy: Owner<T>::Inner<U, Count>::Deep<V>::Leaf.
template <typename T>
class Owner
{
  public:
    template <typename First, int Count, typename Last>
    class Mixed
    {
    };

    class Nested
    {
    };

    enum class Kind
    {
        Item
    };

    template <typename U, int Count>
    class Inner
    {
      public:
        template <typename V>
        class Deep
        {
          public:
            class Leaf
            {
            };
        };
    };
};

class Probe
{
  public:
    // Preserve both template argument lists and apply outer cv only once.
    qualified_names::Owner<double>::Mixed<int, 3, float> nested();
    const volatile qualified_names::Owner<double>::Mixed<int, 3, float> nested_qualified();

    // Ordinary nested types retain and distinguish their owners' arguments.
    Owner<double>::Nested ordinary_double();
    Owner<int>::Nested ordinary_int();

    // Nested enums retain the instantiated owner.
    Owner<double>::Kind kind();

    // Each template segment retains its arguments, including the integer 7.
    Owner<double>::Inner<int, 7>::Deep<long>::Leaf leaf();
};

}  // namespace qualified_names
