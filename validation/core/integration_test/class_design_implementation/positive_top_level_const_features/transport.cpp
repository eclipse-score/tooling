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

template <typename T> struct Box {};

struct Allocator {
  using Handle = int *const;

  void Release(void *p, int count);
  void Store(int *const ptr, const int count);
  void Keep(void *const p);
  void Fill(Box<int *> items);
  const int *const Get();

private:
  int *const buffer_;
};

namespace util {

int Clamp(int value, const int limit);

} // namespace util
