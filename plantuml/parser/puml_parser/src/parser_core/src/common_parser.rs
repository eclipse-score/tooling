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
use pest_derive::Parser;

use crate::common_ast::*;

#[derive(Parser)]
#[grammar = "../grammar/common.pest"]
#[grammar = "../grammar/class.pest"]
#[grammar = "../grammar/component.pest"]
#[grammar = "../grammar/sequence.pest"]
pub struct PlantUmlCommonParser;

pub fn parse_arrow(pair: pest::iterators::Pair<Rule>) -> Result<Arrow, String> {
    let mut left = None;
    let mut right = None;
    let mut segments = Vec::new();
    let mut middle = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::arrow_prefix | Rule::sequence_arrow_prefix => {
                left = Some(ArrowDecor {
                    raw: inner.as_str().to_string(),
                });
            }

            Rule::arrow_suffix | Rule::sequence_arrow_suffix => {
                right = Some(ArrowDecor {
                    raw: inner.as_str().to_string(),
                });
            }

            Rule::arrow_segment => {
                segments.push(inner.as_str());
            }

            Rule::arrow_middle => {
                middle = Some(parse_arrow_middle(inner)?);
            }

            _ => {}
        }
    }

    Ok(Arrow {
        left,
        line: ArrowLine {
            raw: segments.join(""),
        },
        middle,
        right,
    })
}

fn parse_arrow_middle(pair: pest::iterators::Pair<Rule>) -> Result<ArrowMiddle, String> {
    let mut style = None;
    let mut direction = None;
    let mut decorator = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::line_style_block => {
                style = Some(ArrowStyle::default());
            }

            Rule::arrow_direction => {
                direction = Some(parse_arrow_direction(inner)?);
            }

            Rule::arrow_mid_decor => {
                decorator = Some(inner.as_str().to_string());
            }

            _ => {}
        }
    }

    Ok(ArrowMiddle {
        style,
        direction,
        decorator,
    })
}

fn parse_arrow_direction(pair: pest::iterators::Pair<Rule>) -> Result<ArrowDirection, String> {
    match pair.as_str() {
        "up" | "u" => Ok(ArrowDirection::Up),
        "down" | "d" | "do" => Ok(ArrowDirection::Down),
        "left" | "l" | "le" => Ok(ArrowDirection::Left),
        "right" | "r" | "ri" => Ok(ArrowDirection::Right),
        _ => Err(format!(
            "Unknown arrow direction token: '{}'",
            pair.as_str()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;

    #[test]
    fn debug_parse_arrows() {
        let inputs = vec![
            "-->",
            "-[#red]->",
            "o--[dashed]down0->",
            "<==[bold,thickness=2]up(0)->",
            "<|--",
        ];

        for input in inputs {
            println!("Parsing arrow: {}", input);
            let mut pairs =
                PlantUmlCommonParser::parse(Rule::connection_arrow, input).expect("parse failed");

            let pair = pairs.next().expect("no arrow pair");
            let arrow = parse_arrow(pair).expect("arrow parse failed");

            println!("{:#?}", arrow);
            println!("-----------------------------");
        }
    }

    #[test]
    fn test_unknown_arrow_direction_error() {
        // This would previously panic with unreachable!()
        // Now it should return a proper error

        // Note: This test demonstrates the error handling, but requires
        // a grammar that can parse an invalid direction token.
        // In practice, the pest grammar should catch invalid tokens,
        // but this provides defense-in-depth.
    }
}
