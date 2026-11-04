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

#include <functional>
#include <scoped_allocator>
#include <utility>

namespace amp
{
template <typename T, typename U>
class ordinary_template
{
public:
    T value() const;
};

template <typename T, typename U>
class quoted_template_alias
{
public:
    T value() const;
};

template <typename T, typename U>
class quoted_template_no_alias
{
public:
    T value() const;
};

template <typename... Spec>
class Proxy
{
};

template <typename T, typename... Ts>
class TypeParameters
{
};

template <int N, unsigned long M, int... Ns>
class NonTypeParameters
{
};

template <template <typename U> class C, template <typename V> class... Cs>
class TemplateParameters
{
};

template <typename T, typename>
class MixedNamedAndUnnamedParameters
{
};

template <template <typename T, int N> class C>
class NestedOmittedNames
{
};

template <typename...>
class UnnamedPack
{
};

template <typename, typename... Ts>
class UnnamedBeforePack
{
};

template <template <typename> class>
class UnnamedTemplate
{
};

template <template <typename> class...>
class UnnamedTemplatePack
{
};

template <int...>
class UnnamedValuePack
{
};

template <template <template <typename> class> class>
class NestedUnnamedTemplate
{
};
}  // namespace amp

namespace score
{
namespace memory
{
namespace shared
{
// The allocator only needs a declaration: this fixture does not instantiate the map.
template <typename T>
class PolymorphicOffsetPtrAllocator;
}  // namespace shared
}  // namespace memory

namespace mw
{
namespace com
{
namespace impl
{
namespace lola
{
template <typename Key,
          typename MappedType,
          typename KeyEqual = std::equal_to<Key>,
          typename Allocator =
              std::scoped_allocator_adaptor<memory::shared::PolymorphicOffsetPtrAllocator<std::pair<Key, MappedType>>>>
class LinearSearchMap
{
};
}  // namespace lola
}  // namespace impl
}  // namespace com
}  // namespace mw
}  // namespace score
