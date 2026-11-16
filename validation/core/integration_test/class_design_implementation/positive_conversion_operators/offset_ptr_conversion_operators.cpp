/********************************************************************************
 * Copyright (c) 2025-2026 Contributors to the Eclipse Foundation
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
#include <cstddef>
#include <type_traits>

// Extracted from communication score/memory/shared/offset_ptr.h. Keep the
// conversion declarations, bodies and non-void SFINAE helper unchanged.
// Omit unrelated constructors, iterator operations, friends and registry logic:
// this is a declaration/extraction demo, not an offset-pointer implementation.
namespace score::memory::shared {
namespace detail_offset_ptr {
using difference_type = std::ptrdiff_t;
template <typename T, typename PointedType>
using enable_if_type_is_not_void = typename std::enable_if_t<
    std::conjunction_v<std::negation<std::is_same<std::remove_cv_t<T>, void>>, std::is_same<T, PointedType>>>;
constexpr difference_type kNullPtrRepresentation = 1;
}

// Empty stand-in: the real bounds and shared-memory dependencies are not needed
// to extract the conversion signatures. No bounds checking is demonstrated.
struct MemoryRegionBounds {};

template <typename PointedType>
class OffsetPtr {
  public:
    using pointer = PointedType*;
    using difference_type = detail_offset_ptr::difference_type;

    template <class T = PointedType, class = detail_offset_ptr::enable_if_type_is_not_void<T, PointedType>>
    operator pointer() const
    {
        return GetPointerWithBoundsCheck(this, offset_, memory_bounds_, sizeof(PointedType));
    }

    explicit operator bool() const noexcept
    {
        return offset_ != detail_offset_ptr::kNullPtrRepresentation;
    }

  private:
    // Same dependency signature, but deliberately return null instead of
    // reconstructing an address or querying the shared-memory registry.
    static pointer GetPointerWithBoundsCheck(const void* const offset_ptr_address,
                                             const difference_type offset,
                                             const MemoryRegionBounds& offset_ptr_memory_bounds_when_not_in_shm,
                                             const std::size_t pointed_type_size)
    {
        return nullptr;
    }

    // Initialize the stripped demo to null; the real class sets these through
    // its constructors/copy operations, which are outside this fixture's scope.
    detail_offset_ptr::difference_type offset_ = detail_offset_ptr::kNullPtrRepresentation;
    MemoryRegionBounds memory_bounds_;
};
}
