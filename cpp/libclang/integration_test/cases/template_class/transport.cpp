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

template <typename T, int N, typename>
class FixedBuffer
{
  public:
    void push(const T& value);
    T pop();

    // Method template inside class template
    template <typename U>
    U convert(const T& input) const;

    template <typename... Args>
    void emplace(Args&&... args);

    int capacity() const;

  private:
    T m_data[N];
    int m_size;
};

template <template <typename> class Container>
class ContainerOwner
{
  public:
    Container<int> values();
};
