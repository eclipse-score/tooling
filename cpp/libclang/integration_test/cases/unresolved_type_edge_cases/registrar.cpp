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

// `event_control.h` is intentionally never checked in, simulating a header
// that fails to resolve (e.g. a missing dependency).
#include "event_control.h"

#include <map>
#include <vector>

struct Registrar
{
    // Only the parameter type is unresolved; the return type must stay `bool`
    // (clang's own builtin type), not the recovered parameter spelling.
    bool valid_return_bad_param(const EventControl& control);

    // The declarator name ("operator==") is itself made of multiple tokens;
    // recovery must not mistake a token of the type for part of the name.
    bool operator==(const EventControl& other) const;

    // Raw-lexer tokenization emits ">>" as a single token; rendering must
    // still produce correctly spaced/nested output.
    std::map<int, std::vector<EventControl>> nested;

    // Out-of-line definition: the `Registrar::` qualifier must be stripped
    // from the recovered return type.
    EventControl out_of_line();
};

EventControl Registrar::out_of_line()
{
    return {};
}
