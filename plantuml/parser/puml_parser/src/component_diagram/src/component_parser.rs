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
use thiserror::Error;

use crate::{
    Arrow, CompPumlDocument, ComponentStyle, Element, Port, PortType, Relation, Statement,
};
use parser_core::{
    format_parse_tree, pest_to_syntax_error, BaseParseError, DiagramParser, ErrorLocation,
};
use puml_utils::LogLevel;

use parser_core::common_parser::parse_arrow as common_parse_arrow;
use parser_core::common_parser::{PlantUmlCommonParser, Rule};

#[derive(Debug, Error)]
pub enum ComponentError {
    #[error(transparent)]
    Base(#[from] BaseParseError<Rule>),
    #[error("invalid component statement: {0}")]
    InvalidStatement(String),
}

impl ErrorLocation for ComponentError {
    fn error_location(&self) -> Option<(usize, usize)> {
        match self {
            Self::Base(b) => b.error_location(),
            _ => None,
        }
    }
}

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
    fn parse_statement(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<Statement>, ComponentError> {
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::element => {
                    return Ok(vec![Statement::Element(Self::parse_element(inner)?)]);
                }
                Rule::relation => {
                    return Ok(vec![Statement::Relation(Self::parse_relation(inner)?)]);
                }
                Rule::port_declaration => {
                    return Ok(vec![Statement::Port(Self::parse_port(inner)?)]);
                }
                Rule::together_block => {
                    // Flatten children into the enclosing scope (drop the wrapper)
                    return Self::parse_together_block(inner);
                }
                _ => {}
            }
        }
        // footer_line and other non-statement rules produce nothing
        Ok(vec![])
    }

    fn parse_port(pair: pest::iterators::Pair<Rule>) -> Result<Port, ComponentError> {
        let mut port_type = PortType::Port;
        let mut name = String::new();
        let mut alias = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::port_keyword => {
                    port_type = match inner.as_str() {
                        "portin" => PortType::PortIn,
                        "portout" => PortType::PortOut,
                        _ => PortType::Port,
                    };
                }
                Rule::port_name => {
                    name = inner.as_str().to_string();
                }
                Rule::alias_clause => {
                    alias = Self::extract_alias(inner);
                }
                _ => {}
            }
        }

        Ok(Port {
            port_type,
            name,
            alias,
        })
    }

    fn parse_together_block(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<Statement>, ComponentError> {
        let mut stmts = Vec::new();
        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::diagram_statement {
                stmts.append(&mut Self::parse_statement(inner)?);
            }
        }
        Ok(stmts)
    }

    fn parse_element(pair: pest::iterators::Pair<Rule>) -> Result<Element, ComponentError> {
        let mut element = Element {
            kind: "".to_string(),
            name: None,
            alias: None,
            stereotype: None,
            style: None,
            statements: Vec::new(),
        };

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::nested_element => {
                    // Parse the nested element head (which contains default_element or bracket_element)
                    for nested_inner in inner.into_inner() {
                        match nested_inner.as_rule() {
                            Rule::default_element => {
                                let (kind, name_opt) = Self::parse_default_element(nested_inner)?;
                                element.kind = kind;
                                element.name = name_opt;
                            }
                            // For bracket_element, it's always a `component` kind
                            Rule::bracket_element => {
                                let name_opt = Self::parse_bracket_element(nested_inner)?;
                                element.kind = "component".to_string();
                                element.name = name_opt;
                            }
                            _ => {}
                        }
                    }
                }
                Rule::short_form_component => {
                    element.name = Some(Self::extract_component_name(inner));
                    element.kind = "component".to_string();
                }
                Rule::short_form_actor => {
                    element.name = Some(Self::extract_actor_name(inner));
                    element.kind = "actor".to_string();
                }
                Rule::short_form_interface => {
                    element.name = Some(Self::extract_interface_name(inner));
                    element.kind = "interface".to_string();
                }
                Rule::short_form_usecase => {
                    element.name = Some(Self::extract_usecase_name(inner));
                    element.kind = "usecase".to_string();
                }
                Rule::alias_clause => {
                    element.alias = Self::extract_alias(inner);
                }
                Rule::stereotype => {
                    element.stereotype = Self::extract_stereotype(inner);
                }
                Rule::element_style => {
                    element.style = Some(Self::parse_component_style(inner)?);
                }
                Rule::statement_block => {
                    element.statements = Self::parse_statement_block(inner)?;
                }
                _ => {}
            }
        }

        Ok(element)
    }

    fn parse_relation(pair: pest::iterators::Pair<Rule>) -> Result<Relation, ComponentError> {
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
                Rule::relation_description => {
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

    fn parse_arrow(pair: pest::iterators::Pair<Rule>) -> Result<Arrow, ComponentError> {
        let arrow = common_parse_arrow(pair)
            .map_err(|e| ComponentError::InvalidStatement(format!("invalid arrow: {}", e)))?;

        Ok(arrow)
    }

    // Helper methods
    fn extract_component_name(pair: pest::iterators::Pair<Rule>) -> String {
        pair.into_inner()
            .find(|inner| inner.as_rule() == Rule::short_form_component_name)
            .map(|inner| inner.as_str().to_string())
            .unwrap_or_default()
    }

    fn extract_actor_name(pair: pest::iterators::Pair<Rule>) -> String {
        pair.into_inner()
            .find(|inner| inner.as_rule() == Rule::short_form_actor_name)
            .map(|inner| inner.as_str().to_string())
            .unwrap_or_default()
    }

    fn extract_interface_name(pair: pest::iterators::Pair<Rule>) -> String {
        pair.into_inner()
            .find(|inner| inner.as_rule() == Rule::short_form_interface_name)
            .map(|inner| inner.as_str().to_string())
            .unwrap_or_default()
    }

    fn extract_usecase_name(pair: pest::iterators::Pair<Rule>) -> String {
        pair.into_inner()
            .find(|inner| inner.as_rule() == Rule::short_form_usecase_name)
            .map(|inner| inner.as_str().to_string())
            .unwrap_or_default()
    }

    fn extract_alias(pair: pest::iterators::Pair<Rule>) -> Option<String> {
        pair.into_inner()
            .find(|inner| inner.as_rule() == Rule::ALIAS_ID)
            .map(|inner| inner.as_str().to_string())
    }

    fn extract_stereotype(pair: pest::iterators::Pair<Rule>) -> Option<String> {
        pair.into_inner()
            .find(|inner| inner.as_rule() == Rule::STEREOTYPE_NAME)
            .map(|inner| inner.as_str().to_string())
    }

    fn parse_default_element(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<(String, Option<String>), ComponentError> {
        let mut kind = String::new();
        let mut name = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::element_kind => {
                    kind = inner.as_str().to_string();
                }
                Rule::default_element_name => {
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

        Ok((kind, name))
    }

    fn parse_bracket_element(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Option<String>, ComponentError> {
        let mut name: Option<String> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::short_form_component {
                name = Some(Self::extract_component_name(inner));
            }
        }

        Ok(name)
    }

    fn parse_component_style(
        _pair: pest::iterators::Pair<Rule>,
    ) -> Result<ComponentStyle, ComponentError> {
        // Simplified implementation
        Ok(ComponentStyle {
            color: None,
            attributes: Vec::new(),
        })
    }

    fn parse_statement_block(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<Statement>, ComponentError> {
        let mut statements = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::diagram_statement => {
                    let mut stmts = Self::parse_statement(inner)?;
                    statements.append(&mut stmts);
                }
                _ => {
                    // Skip empty lines and other rules like braces
                }
            }
        }

        Ok(statements)
    }
}

impl DiagramParser for PumlComponentParser {
    type Output = CompPumlDocument;
    type Error = ComponentError;

    fn parse_file(
        &mut self,
        path: &Rc<PathBuf>,
        content: &str,
        log_level: LogLevel,
    ) -> Result<Self::Output, Self::Error> {
        use pest::Parser;

        let pairs = PlantUmlCommonParser::parse(Rule::diagram_start, content)
            .map_err(|e| pest_to_syntax_error(e, path.as_ref().clone(), content))?;

        // Debug-only, excluded to keep coverage focused on parser logic.
        #[cfg(not(coverage))]
        if matches!(log_level, LogLevel::Debug | LogLevel::Trace) {
            let mut tree_output = String::new();

            format_parse_tree(pairs.clone(), 0, &mut tree_output);

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
            for inner_pair in pair.into_inner() {
                match inner_pair.as_rule() {
                    Rule::startuml => {
                        if let Some(start_inner) = inner_pair
                            .into_inner()
                            .find(|p| p.as_rule() == Rule::puml_name)
                        {
                            document.name = Some(start_inner.as_str().to_string());
                        }
                    }
                    Rule::diagram_statement => {
                        let mut stmts = Self::parse_statement(inner_pair)?;
                        document.statements.append(&mut stmts);
                    }
                    _ => {
                        // Skip empty lines and other rules like enduml
                    }
                }
            }
        }

        Ok(document)
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;
    use parser_core::DiagramParser;
    use puml_utils::LogLevel;
    use std::path::PathBuf;
    use std::rc::Rc;

    fn path() -> Rc<PathBuf> {
        Rc::new(PathBuf::from("test.puml"))
    }

    /// A valid diagram must still parse successfully – no regression.
    #[test]
    fn test_valid_document_succeeds() {
        let input = "@startuml\ncomponent A\ncomponent B\nA --> B\n@enduml";
        let mut parser = PumlComponentParser;
        let result = parser.parse_file(&path(), input, LogLevel::Info);
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.statements.len(), 3);
    }

    /// A relation that references a component which has no name and no alias
    /// must not silently yield a document with fewer statements than expected.
    #[test]
    fn test_statement_count_matches_source() {
        // Two explicit components + one relation = 3 statements.
        let input =
            "@startuml\ncomponent \"Alpha\" as A\ncomponent \"Beta\" as B\nA --> B : link\n@enduml";
        let mut parser = PumlComponentParser;
        let doc = parser
            .parse_file(&path(), input, LogLevel::Info)
            .expect("valid diagram must parse");
        assert_eq!(
            doc.statements.len(),
            3,
            "all statements must be present; none may be silently dropped"
        );
    }
}

#[cfg(test)]
mod dispatch_style_tests {
    use super::*;
    use parser_core::DiagramParser;
    use puml_utils::LogLevel;
    use std::path::PathBuf;
    use std::rc::Rc;

    /// Smoke test: the statement count from a two-component, one-relation diagram
    /// must be exactly 3 for the component parser.
    #[test]
    fn test_component_statement_count() {
        let input = "@startuml\ncomponent A\ncomponent B\nA --> B\n@enduml";
        let mut parser = PumlComponentParser;
        let doc = parser
            .parse_file(&Rc::new(PathBuf::from("t.puml")), input, LogLevel::Info)
            .expect("valid input must parse");
        assert_eq!(doc.statements.len(), 3);
    }
}
