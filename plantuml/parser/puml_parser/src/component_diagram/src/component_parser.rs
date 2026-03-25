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
use log::debug;
use std::path::PathBuf;
use std::rc::Rc;

use crate::{Arrow, CompPumlDocument, Component, ComponentStyle, Relation, Statement};
use parser_core::{pest_to_syntax_error, BaseParseError, DiagramParser};
use puml_utils::LogLevel;

use parser_core::common_parser::parse_arrow as common_parse_arrow;
use parser_core::common_parser::{PlantUmlCommonParser, Rule};

pub struct PumlComponentParser;

// lobster-trace: Tools.ArchitectureModelingSyntax
// lobster-trace: Tools.ArchitectureModelingComponentContentComponent
// lobster-trace: Tools.ArchitectureModelingComponentContentSEooC
// lobster-trace: Tools.ArchitectureModelingComponentContentSWUnit
// lobster-trace: Tools.ArchitectureModelingComponentContentAbstractInterface
// lobster-trace: Tools.ArchitectureModelingComponentHierarchySEooC
// lobster-trace: Tools.ArchitectureModelingComponentHierarchyComponent
// lobster-trace: Tools.ArchitectureModelingComponentInteract
impl PumlComponentParser {
    fn format_parse_tree(pairs: pest::iterators::Pairs<Rule>, indent: usize, output: &mut String) {
        for pair in pairs {
            let indent_str = "  ".repeat(indent);

            output.push_str(&format!(
                "{}Rule::{:?} -> \"{}\"\n",
                indent_str,
                pair.as_rule(),
                pair.as_str()
            ));

            // Recursively print inner pairs
            Self::format_parse_tree(pair.into_inner(), indent + 1, output);
        }
    }

    fn parse_statement(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Statement, Box<dyn std::error::Error>> {
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::component => {
                    return Ok(Statement::Component(Self::parse_component(inner)?));
                }
                Rule::relation => {
                    return Ok(Statement::Relation(Self::parse_relation(inner)?));
                }
                _ => {}
            }
        }
        Err("Invalid statement".into())
    }

    fn parse_component(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Component, Box<dyn std::error::Error>> {
        let mut component = Component {
            component_type: "".to_string(),
            name: None,
            alias: None,
            stereotype: None,
            style: None,
            statements: Vec::new(),
        };

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::nested_component => {
                    // Parse the nested component (which contains default_component or bracket_component)
                    for nested_inner in inner.into_inner() {
                        match nested_inner.as_rule() {
                            Rule::default_component => {
                                let (ctype, name_opt) =
                                    Self::parse_default_component(nested_inner)?;
                                component.component_type = ctype;
                                component.name = name_opt;
                            }
                            // For bracket_component, it's always a `component` type
                            Rule::bracket_component => {
                                let name_opt = Self::parse_bracket_component(nested_inner)?;
                                component.component_type = "component".to_string();
                                component.name = name_opt;
                            }
                            _ => {}
                        }
                    }
                }
                Rule::component_old => {
                    component.name = Some(Self::extract_component_name(inner));
                    component.component_type = "component".to_string();
                }
                Rule::interface_old => {
                    component.name = Some(Self::extract_interface_name(inner));
                    component.component_type = "interface".to_string();
                }
                Rule::default_component => {
                    let (ctype, name_opt) = Self::parse_default_component(inner)?;
                    component.component_type = ctype;
                    component.name = name_opt;
                }
                Rule::bracket_component => {
                    let name_opt = Self::parse_bracket_component(inner)?;
                    component.component_type = "component".to_string();
                    component.name = name_opt;
                }
                Rule::alias_clause => {
                    component.alias = Self::extract_alias(inner);
                }
                Rule::stereotype => {
                    component.stereotype = Self::extract_stereotype(inner);
                }
                Rule::component_style => {
                    component.style = Some(Self::parse_component_style(inner)?);
                }
                Rule::statement_block => {
                    component.statements = Self::parse_statement_block(inner)?;
                }
                _ => {}
            }
        }

        Ok(component)
    }

    fn parse_relation(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Relation, Box<dyn std::error::Error>> {
        let mut lhs = String::new();
        let mut rhs = String::new();
        let mut arrow = Arrow::default();

        let mut description = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::relation_left => {
                    lhs = inner.as_str().to_string();
                }
                Rule::relation_right => {
                    rhs = inner.as_str().to_string();
                }
                Rule::connection_arrow => {
                    arrow = Self::parse_arrow(inner)?;
                }
                Rule::component_description => {
                    description = Self::parse_description(inner);
                }
                _ => {}
            }
        }

        Ok(Relation {
            lhs,
            arrow,
            rhs,
            style: None,
            description,
        })
    }

    fn parse_description(pair: pest::iterators::Pair<Rule>) -> Option<String> {
        pair.into_inner()
            .find(|p| p.as_rule() == Rule::description_text)
            .map(|p| p.as_str().trim().to_string())
    }

    fn parse_arrow(pair: pest::iterators::Pair<Rule>) -> Result<Arrow, Box<dyn std::error::Error>> {
        let arrow = common_parse_arrow(pair).map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                as Box<dyn std::error::Error>
        })?;

        Ok(arrow)
    }

    // Helper methods
    fn extract_component_name(pair: pest::iterators::Pair<Rule>) -> String {
        for inner in pair.into_inner() {
            if let Rule::component_old_name = inner.as_rule() {
                return inner.as_str().to_string();
            }
        }
        String::new()
    }

    fn extract_interface_name(pair: pest::iterators::Pair<Rule>) -> String {
        for inner in pair.into_inner() {
            if let Rule::interface_old_name = inner.as_rule() {
                return inner.as_str().to_string();
            }
        }
        String::new()
    }

    fn extract_alias(pair: pest::iterators::Pair<Rule>) -> Option<String> {
        for inner in pair.into_inner() {
            if let Rule::ALIAS_ID = inner.as_rule() {
                return Some(inner.as_str().to_string());
            }
        }
        None
    }

    fn extract_stereotype(pair: pest::iterators::Pair<Rule>) -> Option<String> {
        for inner in pair.into_inner() {
            if let Rule::STEREOTYPE_NAME = inner.as_rule() {
                return Some(inner.as_str().to_string());
            }
        }
        None
    }

    fn parse_default_component(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
        let mut comp_type = String::new();
        let mut name = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::component_type => {
                    comp_type = inner.as_str().to_string();
                }
                Rule::default_component_name => {
                    let raw_name = inner.as_str().to_string();
                    // Remove surrounding quotes if present
                    let clean_name = if raw_name.starts_with('"') && raw_name.ends_with('"') {
                        raw_name[1..raw_name.len() - 1].to_string()
                    } else {
                        raw_name
                    };
                    name = Some(clean_name);
                }
                _ => {}
            }
        }

        Ok((comp_type, name))
    }

    fn parse_bracket_component(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let mut name: Option<String> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::component_old {
                name = Some(Self::extract_component_name(inner));
            }
        }

        Ok(name)
    }

    fn parse_component_style(
        _pair: pest::iterators::Pair<Rule>,
    ) -> Result<ComponentStyle, Box<dyn std::error::Error>> {
        // Simplified implementation
        Ok(ComponentStyle {
            color: None,
            attributes: Vec::new(),
        })
    }

    fn parse_statement_block(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<Statement>, Box<dyn std::error::Error>> {
        let mut statements = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::component_statement => {
                    if let Ok(stmt) = Self::parse_statement(inner) {
                        statements.push(stmt);
                    }
                }
                Rule::EOL => {
                    // Skip empty lines
                }
                _ => {
                    // Skip other rules like braces
                }
            }
        }

        Ok(statements)
    }
}

impl DiagramParser for PumlComponentParser {
    type Output = CompPumlDocument;
    type Error = BaseParseError<Rule>;

    fn parse_file(
        &mut self,
        path: &Rc<PathBuf>,
        content: &str,
        log_level: LogLevel,
    ) -> Result<Self::Output, Self::Error> {
        use pest::Parser;

        let pairs = PlantUmlCommonParser::parse(Rule::component_start, content)
            .map_err(|e| pest_to_syntax_error(e, path.as_ref().clone(), content))?;

        // Show raw parse tree at debug level
        if matches!(log_level, LogLevel::Debug | LogLevel::Trace) {
            let mut tree_output = String::new();

            Self::format_parse_tree(pairs.clone(), 0, &mut tree_output);

            debug!(
                "\n=== Parse Tree for {} ===\n{}=== End Parse Tree ===",
                path.display(),
                tree_output
            );
        }

        let mut document = CompPumlDocument {
            name: None,
            statements: Vec::new(),
        };

        for pair in pairs {
            if pair.as_rule() == Rule::component_start {
                for inner_pair in pair.into_inner() {
                    match inner_pair.as_rule() {
                        Rule::startuml => {
                            for start_inner in inner_pair.into_inner() {
                                if let Rule::puml_name = start_inner.as_rule() {
                                    document.name = Some(start_inner.as_str().to_string());
                                }
                            }
                        }
                        Rule::component_statement => {
                            if let Ok(stmt) = Self::parse_statement(inner_pair) {
                                document.statements.push(stmt);
                            }
                        }
                        Rule::empty_line => {
                            // Skip empty lines
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(document)
    }
}
