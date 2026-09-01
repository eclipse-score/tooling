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

bool is_ready();
bool is_allowed();
bool has_permission();
void handle_complex();

namespace flow {
void complex_condition() {
    if (is_ready() && (is_allowed() || !has_permission())) {
        handle_complex();
    }
}
}  // namespace flow
