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
#include <cstring>

#include "src/coverable.h"

// Deliberately exercises only the negative and zero branches; the positive
// branch stays uncovered (and justified via the COV_JUSTIFIED marker).
int main() {
  using coverage_integration::classify;
  if (std::strcmp(classify(-5), "negative") != 0) {
    return 1;
  }
  if (std::strcmp(classify(0), "zero") != 0) {
    return 1;
  }
  return 0;
}
