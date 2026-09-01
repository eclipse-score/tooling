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

void handle_outer_else();
void handle_nested();

namespace flow {
void nested_if(bool outer, bool inner) {
    if (outer) {
        if (inner) {
            handle_nested();
        }
    } else {
        handle_outer_else();
    }
}
}  // namespace flow
