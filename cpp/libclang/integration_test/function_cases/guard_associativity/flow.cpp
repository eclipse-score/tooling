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

bool first();
bool second();
bool third();
void handle_and();
void handle_or();
void handle_mixed();
void handle_alternative();
void handle_template_operand();
void handle_operator_function();

template <bool>
bool check_template();

constexpr bool template_first = true;
constexpr bool template_second = true;

struct Flag {};
Flag first_flag();
Flag second_flag();
bool left;
bool operator&&(Flag, Flag);

namespace flow {
void guard_associativity() {
    // Flattens a parenthesized right-associative logical-and chain.
    if (first() && (second() && third())) {
        handle_and();
    }

    // Preserves an or expression nested in a logical-and expression.
    if ((first() || second()) && third()) {
        handle_or();
    }

    // Uses C++ precedence for an unparenthesized && / || expression.
    if (first() && second() || third()) {
        handle_mixed();
    }

    // Recognizes C++'s alternative logical-and token.
    if (first() and second()) {
        handle_alternative();
    }

    // Ignores a logical token inside a template argument of a non-logical root.
    if (check_template<template_first && template_second>() == third()) {
        handle_template_operand();
    }

    // Does not mistake operator&& in the right-hand CallExpr for the outer ==.
    if (left == operator&&(first_flag(), second_flag())) {
        handle_operator_function();
    }
}
}  // namespace flow
