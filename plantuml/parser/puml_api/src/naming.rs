// *******************************************************************************
// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// <https://www.apache.org/licenses/LICENSE-2.0>
//
// SPDX-License-Identifier: Apache-2.0
// *******************************************************************************

//! TRLC record names of API items.

use class_diagram::MethodModifier;
use puml_utils::is_trlc_identifier;

use crate::ApiError;

/// Record name part of the C++ operator `symbol` (the text after `operator`).
fn operator_name(symbol: &str) -> Option<&'static str> {
    Some(match symbol {
        "->*" => "arrow_star",
        "->" => "arrow",
        "()" => "call",
        "[]" => "subscript",
        "<=>" => "spaceship",
        "<<" => "shl",
        ">>" => "shr",
        "<=" => "le",
        ">=" => "ge",
        "==" => "eq",
        "!=" => "ne",
        "&&" => "and",
        "||" => "or",
        "+=" => "add_assign",
        "-=" => "sub_assign",
        "*=" => "mul_assign",
        "/=" => "div_assign",
        "%=" => "mod_assign",
        "^=" => "xor_assign",
        "&=" => "and_assign",
        "|=" => "or_assign",
        "++" => "inc",
        "--" => "dec",
        "+" => "add",
        "-" => "sub",
        "*" => "mul",
        "/" => "div",
        "%" => "mod",
        "^" => "xor",
        "&" => "bitand",
        "|" => "bitor",
        "~" => "compl",
        "!" => "not",
        "=" => "assign",
        "<" => "lt",
        ">" => "gt",
        "," => "comma",
        _ => return None,
    })
}

/// Member part of a record name: `destructor`, `operator_<name>` or the method
/// name (a `::` qualification becomes `_`).
pub(crate) fn method_part(
    name: &str,
    modifiers: &[MethodModifier],
    origin: &str,
) -> Result<String, ApiError> {
    if modifiers.contains(&MethodModifier::Destructor) {
        return Ok("destructor".to_string());
    }
    if let Some(rest) = name.strip_prefix("operator") {
        // `operatorX` is a plain identifier; any other continuation (a symbol or
        // a separated keyword/type as in `operator bool`) names an operator.
        let is_operator = rest
            .chars()
            .next()
            .is_some_and(|c| c.is_whitespace() || !(c.is_ascii_alphanumeric() || c == '_'));
        if is_operator {
            let symbol = rest.trim();
            return match operator_name(symbol) {
                Some(part) => Ok(format!("operator_{part}")),
                None => Err(ApiError::UnsupportedOperator {
                    origin: origin.to_string(),
                    operator: symbol.to_string(),
                }),
            };
        }
    }
    Ok(name.replace("::", "_"))
}

/// Record name `<scope>_<member>` (`Free_<member>` without a scope); checks
/// that it is a valid TRLC identifier.
pub(crate) fn record_name(
    scope: Option<&str>,
    member: &str,
    origin: &str,
) -> Result<String, ApiError> {
    let name = match scope {
        Some(scope) => format!("{}_{member}", scope.replace('.', "_")),
        None => format!("Free_{member}"),
    };
    if is_trlc_identifier(&name) {
        Ok(name)
    } else {
        Err(ApiError::InvalidName {
            origin: origin.to_string(),
            part: name,
        })
    }
}

/// Key under which TRLC compares record names: ignoring case and `_`.
pub(crate) fn name_key(record: &str) -> String {
    record.to_lowercase().replace('_', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_table_has_no_duplicate_names() {
        let symbols = [
            "->*", "->", "()", "[]", "<=>", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "+=",
            "-=", "*=", "/=", "%=", "^=", "&=", "|=", "++", "--", "+", "-", "*", "/", "%", "^",
            "&", "|", "~", "!", "=", "<", ">", ",",
        ];
        let mut seen = std::collections::HashSet::new();
        for symbol in symbols {
            let name = operator_name(symbol).expect("operator is in the table");
            assert!(seen.insert(name), "duplicate operator name {name}");
        }
    }

    #[test]
    fn name_key_ignores_case_and_underscores() {
        assert_eq!(name_key("A_get_X"), name_key("agetx"));
    }

    #[test]
    fn method_part_names_operators_and_plain_identifiers() {
        for (name, part) in [
            ("operator()", "operator_call"),
            ("operator ==", "operator_eq"),
            ("operatorX", "operatorX"),
            ("operator_x", "operator_x"),
            ("ns::f", "ns_f"),
        ] {
            assert_eq!(method_part(name, &[], "A::f").as_deref(), Ok(part));
        }
    }

    #[test]
    fn method_part_rejects_operators_without_a_record_name() {
        for (name, operator) in [
            ("operator bool", "bool"),
            ("operator new", "new"),
            ("operator delete[]", "delete[]"),
            ("operator co_await", "co_await"),
        ] {
            assert_eq!(
                method_part(name, &[], "A::f"),
                Err(ApiError::UnsupportedOperator {
                    origin: "A::f".to_string(),
                    operator: operator.to_string(),
                })
            );
        }
    }
}
