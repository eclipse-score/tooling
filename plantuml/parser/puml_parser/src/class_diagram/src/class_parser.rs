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
use crate::class_ast::{
    Arrow, Attribute, ClassDef, ClassUmlFile, ClassUmlTopLevel, Element, EnumDef, EnumItem,
    EnumValue, InterfaceDef, Method, Name, Namespace, Package, Param, Relationship, StructDef,
    Visibility,
};
use crate::class_traits::{TypeDef, WritableName};
use parser_core::common_parser::{parse_arrow, PlantUmlCommonParser, Rule};
use parser_core::{pest_to_syntax_error, BaseParseError, DiagramParser};
use pest::Parser;
use puml_utils::LogLevel;
use std::path::PathBuf;
use std::rc::Rc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClassError {
    #[error(transparent)]
    Base(#[from] BaseParseError<Rule>),
}

fn parse_visibility(pair: Option<pest::iterators::Pair<Rule>>) -> Visibility {
    let mut vis = Visibility::Public;
    if let Some(v) = pair {
        match v.as_str() {
            "+" => vis = Visibility::Public,
            "-" => vis = Visibility::Private,
            "#" => vis = Visibility::Protected,
            "~" => vis = Visibility::Package,
            _ => (),
        }
    }
    vis
}

fn parse_named(pair: pest::iterators::Pair<Rule>, name: &mut Name) {
    let mut internal: Option<String> = None;
    let mut display: Option<String> = None;

    fn strip_quotes(s: &str) -> String {
        if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
            s[1..s.len() - 1].to_string()
        } else {
            s.to_string()
        }
    }

    fn walk(
        pair: pest::iterators::Pair<Rule>,
        internal: &mut Option<String>,
        display: &mut Option<String>,
    ) {
        match pair.as_rule() {
            Rule::internal_name => {
                *internal = Some(strip_quotes(pair.as_str()));
            }
            Rule::alias_clause => {
                let mut inner = pair.into_inner();
                if let Some(target) = inner.next() {
                    *display = Some(strip_quotes(target.as_str()));
                }
            }
            _ => {
                for inner in pair.into_inner() {
                    walk(inner, internal, display);
                }
            }
        }
    }

    walk(pair, &mut internal, &mut display);

    if let Some(internal) = internal {
        name.write_name(&internal, display.as_deref());
    }
}

fn parse_attribute(pair: pest::iterators::Pair<Rule>) -> Attribute {
    let mut attr = Attribute::default();
    let mut vis = None;
    let mut name = None;
    let mut typ = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::class_visibility => vis = Some(p),
            Rule::identifier => name = Some(p.as_str().to_string()),
            Rule::type_name => typ = Some(p.as_str().to_string()),
            _ => {}
        }
    }

    attr.visibility = parse_visibility(vis);
    attr.name = name.unwrap_or_default();
    attr.r#type = typ;
    attr
}

fn parse_param(pair: pest::iterators::Pair<Rule>) -> Param {
    let mut name: Option<String> = None;
    let mut ty: Option<String> = None;
    let mut varargs = false;

    // param -> param_named | param_unnamed
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::param_named => {
            for p in inner.into_inner() {
                match p.as_rule() {
                    Rule::identifier => {
                        name = Some(p.as_str().to_string());
                    }
                    Rule::type_name => {
                        ty = Some(p.as_str().to_string());
                    }
                    Rule::varargs => {
                        varargs = true;
                    }
                    _ => {}
                }
            }
        }

        Rule::param_unnamed => {
            for p in inner.into_inner() {
                match p.as_rule() {
                    Rule::type_name => {
                        ty = Some(p.as_str().to_string());
                    }
                    Rule::varargs => {
                        varargs = true;
                    }
                    _ => {}
                }
            }
        }

        _ => unreachable!(),
    }

    Param {
        name,
        param_type: ty.expect("param must have a type"),
        varargs,
    }
}

fn parse_method(pair: pest::iterators::Pair<Rule>) -> Method {
    fn parse_generic_param_list(pair: pest::iterators::Pair<Rule>) -> Vec<String> {
        pair.into_inner()
            .filter(|p| p.as_rule() == Rule::identifier)
            .map(|p| p.as_str().to_string())
            .collect()
    }

    let mut method = Method::default();
    let mut vis = None;
    let mut name = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::class_visibility => vis = Some(p),
            Rule::identifier => name = Some(p.as_str().to_string()),
            Rule::param_list => {
                for param_pair in p.into_inner() {
                    if param_pair.as_rule() == Rule::param {
                        let param = parse_param(param_pair);
                        method.params.push(param);
                    }
                }
            }
            Rule::return_type => {
                for return_type_inner in p.into_inner() {
                    if return_type_inner.as_rule() == Rule::type_name {
                        method.r#type = Some(return_type_inner.as_str().to_string());
                    }
                }
            }
            Rule::generic_param_list => {
                method.generic_params = parse_generic_param_list(p);
            }
            _ => (),
        }
    }
    method.visibility = parse_visibility(vis);
    method.name = name.unwrap_or_default();

    method
}

fn parse_type_def_into<T>(pair: pest::iterators::Pair<Rule>) -> T
where
    T: TypeDef + Default,
{
    let mut def = T::default();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::named => {
                parse_named(p, def.name_mut());
            }
            Rule::class_body => {
                for inner in p.into_inner() {
                    if let Rule::class_member = inner.as_rule() {
                        for member in inner.into_inner() {
                            match member.as_rule() {
                                Rule::attribute => {
                                    def.attributes_mut().push(parse_attribute(member))
                                }
                                Rule::method => def.methods_mut().push(parse_method(member)),
                                _ => (),
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }

    def
}

fn parse_type_def(pair: pest::iterators::Pair<Rule>) -> Element {
    debug_assert_eq!(pair.as_rule(), Rule::type_def);

    let mut inner = pair.clone().into_inner();

    let kind_pair = inner.next().expect("type_def must have type_kind");

    let kind = kind_pair.as_str(); // "class" | "struct"

    match kind {
        "class" => Element::ClassDef(parse_type_def_into::<ClassDef>(pair)),
        "struct" => Element::StructDef(parse_type_def_into::<StructDef>(pair)),
        "interface" => Element::InterfaceDef(parse_type_def_into::<InterfaceDef>(pair)),
        _ => unreachable!("unknown type_kind: {}", kind),
    }
}

fn parse_enum_def(pair: pest::iterators::Pair<Rule>) -> EnumDef {
    let mut enum_def = EnumDef::default();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::named => {
                // enum_def.name = inner.as_str().trim().to_string();
                parse_named(inner, &mut enum_def.name);
            }
            Rule::enum_body => {
                enum_def.items = parse_enum_body(inner);
            }
            _ => (),
        }
    }

    enum_def
}

fn parse_enum_body(pair: pest::iterators::Pair<Rule>) -> Vec<EnumItem> {
    pair.into_inner()
        .filter(|p| p.as_rule() == Rule::enum_item)
        .map(parse_enum_item)
        .collect()
}

fn parse_enum_item(pair: pest::iterators::Pair<Rule>) -> EnumItem {
    let mut item = EnumItem::default();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::class_visibility => {
                item.visibility = Some(parse_visibility(Some(inner)));
            }
            Rule::identifier => {
                item.name = inner.as_str().to_string();
            }
            Rule::enum_value => {
                item.value = Some(parse_enum_value(inner));
            }
            _ => (),
        }
    }

    item
}

fn parse_enum_value(pair: pest::iterators::Pair<Rule>) -> EnumValue {
    let text = pair.as_str().trim();

    if let Some(rest) = text.strip_prefix('=') {
        EnumValue::Literal(rest.trim().to_string())
    } else if let Some(rest) = text.strip_prefix(':') {
        EnumValue::Description(rest.trim().to_string())
    } else {
        EnumValue::Literal(text.to_string())
    }
}

fn parse_namespace(pair: pest::iterators::Pair<Rule>) -> Namespace {
    let mut namespace = Namespace::default();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::named => {
                parse_named(inner, &mut namespace.name);
            }
            Rule::top_level => {
                for top_level_inner in inner.into_inner() {
                    match top_level_inner.as_rule() {
                        Rule::type_def => {
                            let mut type_def = parse_type_def(top_level_inner);
                            type_def.set_namespace(namespace.name.internal.clone());
                            namespace.types.push(type_def);
                        }
                        Rule::enum_def => {
                            let mut enum_def = Element::EnumDef(parse_enum_def(top_level_inner));
                            enum_def.set_namespace(namespace.name.internal.clone());
                            namespace.types.push(enum_def);
                        }
                        Rule::namespace_def => {
                            namespace.namespaces.push(parse_namespace(top_level_inner));
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }
    }

    namespace
}

fn parse_package(pair: pest::iterators::Pair<Rule>) -> Package {
    let mut package = Package::default();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::named => {
                parse_named(inner, &mut package.name);
            }

            Rule::top_level => {
                for t in inner.into_inner() {
                    match t.as_rule() {
                        Rule::type_def => {
                            let mut r#type = parse_type_def(t);
                            r#type.set_package(package.name.internal.clone());
                            package.types.push(r#type);
                        }
                        Rule::enum_def => {
                            let mut enum_def = Element::EnumDef(parse_enum_def(t));
                            enum_def.set_package(package.name.internal.clone());
                            package.types.push(enum_def);
                        }
                        Rule::relationship => {
                            package.relationships.push(parse_relationship(t));
                        }
                        Rule::package_def => {
                            package.packages.push(parse_package(t));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    package
}

fn parse_label(pair: pest::iterators::Pair<Rule>) -> String {
    pair.as_str().trim().to_string()
}

fn parse_relationship(pair: pest::iterators::Pair<Rule>) -> Relationship {
    let mut inner = pair.into_inner();

    let left = inner.next().unwrap().as_str().trim().to_string();

    let arrow_pair = inner.next().unwrap();
    let arrow = parse_arrow(arrow_pair).unwrap_or_else(|_| Arrow::default());

    let right = inner.next().unwrap().as_str().trim().to_string();

    let mut label: Option<String> = None;
    for p in inner {
        if p.as_rule() == Rule::label {
            label = Some(parse_label(p));
        }
    }

    Relationship {
        left,
        right,
        arrow,
        label,
    }
}

/// Parser struct for class diagrams
pub struct PumlClassParser;

impl DiagramParser for PumlClassParser {
    type Output = ClassUmlFile;
    type Error = ClassError;

    fn parse_file(
        &mut self,
        path: &Rc<PathBuf>,
        content: &str,
        log_level: LogLevel,
    ) -> Result<Self::Output, Self::Error> {
        // Log file content at trace level
        if matches!(log_level, LogLevel::Trace) {
            eprintln!("{}:\n{}\n{}", path.display(), content, "=".repeat(30));
        }

        let mut uml_file = ClassUmlFile::default();

        match PlantUmlCommonParser::parse(Rule::class_start, content) {
            Ok(mut pairs) => {
                let file_pair = pairs.next().unwrap();

                let inner = file_pair.into_inner();

                for pair in inner {
                    match pair.as_rule() {
                        Rule::top_level => {
                            for inner_pair in pair.into_inner() {
                                match inner_pair.as_rule() {
                                    Rule::type_def => {
                                        let type_def = parse_type_def(inner_pair);
                                        uml_file.elements.push(ClassUmlTopLevel::Types(type_def));
                                    }
                                    Rule::enum_def => {
                                        uml_file.elements.push(ClassUmlTopLevel::Enum(
                                            parse_enum_def(inner_pair),
                                        ));
                                    }
                                    Rule::namespace_def => {
                                        uml_file.elements.push(ClassUmlTopLevel::Namespace(
                                            parse_namespace(inner_pair),
                                        ));
                                    }
                                    Rule::relationship => {
                                        uml_file.relationships.push(parse_relationship(inner_pair));
                                    }
                                    Rule::package_def => {
                                        uml_file.elements.push(ClassUmlTopLevel::Package(
                                            parse_package(inner_pair),
                                        ));
                                    }
                                    _ => (),
                                }
                            }
                        }
                        Rule::startuml => {
                            let text = pair.as_str();
                            if let Some(name) = text.split_whitespace().nth(1) {
                                uml_file.name = name.to_string();
                            }
                        }
                        _ => (),
                    }
                }
            }
            Err(e) => {
                return Err(ClassError::Base(pest_to_syntax_error(
                    e,
                    path.as_ref().clone(),
                    content,
                )));
            }
        };

        Ok(uml_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_visibility_none() {
        let vis = super::parse_visibility(None);
        assert_eq!(vis, Visibility::Public);
    }

    #[test]
    fn test_parse_visibility_unknown_symbol() {
        let pair = PlantUmlCommonParser::parse(Rule::identifier, "abc")
            .unwrap()
            .next()
            .unwrap();

        let vis = super::parse_visibility(Some(pair));

        assert_eq!(vis, Visibility::Public);
    }

    #[test]
    fn test_parse_param_unnamed_varargs() {
        let input = "int...";
        let pair = PlantUmlCommonParser::parse(Rule::param, input)
            .unwrap()
            .next()
            .unwrap();

        let param = super::parse_param(pair);

        assert_eq!(param.name, None);
        assert_eq!(param.param_type, "int");
        assert!(param.varargs);
    }

    #[test]
    fn test_parse_file_error() {
        let mut parser = PumlClassParser;

        let result = parser.parse_file(
            &std::rc::Rc::new(std::path::PathBuf::from("test.puml")),
            "invalid syntax !!!",
            LogLevel::Info,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_attribute_without_name() {
        let input = r#"@startuml
            class A {
                +a
            }
            @enduml
        "#;

        let mut parser = PumlClassParser;
        let result = parser
            .parse_file(
                &std::rc::Rc::new(std::path::PathBuf::from("test.puml")),
                input,
                LogLevel::Info,
            )
            .unwrap();

        assert!(!result.elements.is_empty());
    }

    #[test]
    fn test_parse_relationship_minimal() {
        let pair = PlantUmlCommonParser::parse(Rule::relationship, "A --> B")
            .unwrap()
            .next()
            .unwrap();

        let rel = super::parse_relationship(pair);

        assert_eq!(rel.left, "A");
        assert_eq!(rel.right, "B");
    }

    #[test]
    fn test_enum_value_all_cases() {
        // literal
        let pair = PlantUmlCommonParser::parse(Rule::enum_value, "= 1")
            .unwrap()
            .next()
            .unwrap();
        match super::parse_enum_value(pair) {
            EnumValue::Literal(v) => assert_eq!(v, "1"),
            _ => panic!(),
        }

        // description
        let pair = PlantUmlCommonParser::parse(Rule::enum_value, ": ok")
            .unwrap()
            .next()
            .unwrap();
        match super::parse_enum_value(pair) {
            EnumValue::Description(v) => assert_eq!(v, "ok"),
            _ => panic!(),
        }
    }
}
