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

#include "functions.hpp"

void alias_fn(int value) {}
void typedef_fn(int value) {}
void arr_fn(int* values) {}
void incomplete_arr_fn(int* values) {}
void callback_fn(void (*callback)(int)) {}
void callback_noexcept_identity(void (*)()) {}
void callback_noexcept_identity(void (*)() noexcept) {}
