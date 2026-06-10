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
#ifndef TOOLS_CPP_LIBCLANG_INTEGRATION_TESTS_CASES_INCLUDE_3RDPARTY_TRANSPORT_H
#define TOOLS_CPP_LIBCLANG_INTEGRATION_TESTS_CASES_INCLUDE_3RDPARTY_TRANSPORT_H
#include "flatbuffers/flatbuffers.h"
#include <cstdint>
#include <vector>

class Car
{
  public:
    flatbuffers::Offset<flatbuffers::String> getNameOffset() const;
    flatbuffers::FlatBufferBuilder createBuilder();

    std::uint64_t m_doors;
    std::vector<int> m_windows;
    flatbuffers::FlatBufferBuilder m_builder;

  protected:
    std::vector<flatbuffers::Offset<flatbuffers::String>> m_name_offsets;

  private:
    std::vector<flatbuffers::uoffset_t> m_offsets;
};
#endif  // TOOLS_CPP_LIBCLANG_INTEGRATION_TESTS_CASES_INCLUDE_3RDPARTY_TRANSPORT_H
