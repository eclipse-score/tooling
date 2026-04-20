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
    EnumValue, InterfaceDef, Method, Name, Namespace, ObjectDef, Package, Param, Relationship,
    StructDef, Visibility,
};
use crate::class_traits::{TypeDef, WritableName};
use crate::source_map::{normalize_multiline_member_signatures, remap_syntax_error_to_original_source};
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
                let raw = pair.as_str().to_string();
                let saw_inner = pair.clone().into_inner().next().is_some();

                for inner in pair.into_inner() {
                    walk(inner, internal, display);
                }

                if !saw_inner {
                    *internal = Some(strip_quotes(&raw));
                }
            }
            Rule::STRING | Rule::class_qualified_name => {
                if internal.is_none() {
                    *internal = Some(strip_quotes(pair.as_str()));
                }
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
            Rule::static_modifier => attr.modifiers.push(p.as_str().to_string()),
            Rule::class_visibility => vis = Some(p),
            Rule::using_attribute => {
                for inner in p.into_inner() {
                    match inner.as_rule() {
                        Rule::identifier => name = Some(inner.as_str().to_string()),
                        Rule::type_name => typ = Some(inner.as_str().trim().to_string()),
                        _ => {}
                    }
                }
            }
            Rule::named_attribute => {
                for inner in p.into_inner() {
                    match inner.as_rule() {
                        Rule::identifier => name = Some(inner.as_str().to_string()),
                        Rule::type_name => typ = Some(inner.as_str().trim().to_string()),
                        _ => {}
                    }
                }
            }
            Rule::unnamed_attribute => {
                for inner in p.into_inner() {
                    if inner.as_rule() == Rule::type_name {
                        typ = Some(inner.as_str().trim().to_string());
                    }
                }
            }
            _ => {} // LCOV_EXCL_LINE
        }
    }

    attr.visibility = parse_visibility(vis);
    attr.name = name.unwrap_or_default();
    attr.r#type = typ;
    attr
}

fn parse_param(pair: pest::iterators::Pair<Rule>) -> Param {
    fn is_likely_type_only_param(raw: &str) -> bool {
        const PRIMITIVE_TYPES: &[&str] = &[
            "bool", "char", "short", "int", "long", "float", "double", "void", "size_t",
            "ssize_t", "uint8", "uint16", "uint32", "uint64", "int8", "int16", "int32",
            "int64", "auto",
        ];

        let trimmed = raw.trim();

        if trimmed.is_empty() {
            return false;
        }

        if PRIMITIVE_TYPES.contains(&trimmed) {
            return true;
        }

        if trimmed.starts_with("const ")
            || trimmed.contains("::")
            || trimmed.contains('.')
            || trimmed.contains('<')
            || trimmed.contains('>')
            || trimmed.contains('&')
            || trimmed.contains('*')
            || trimmed.contains('[')
            || trimmed.contains(']')
            || trimmed.contains('{')
            || trimmed.contains('}')
            || trimmed.contains('(')
            || trimmed.contains(')')
        {
            return true;
        }

        trimmed.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
    }

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
                        ty = Some(p.as_str().trim().to_string());
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
                        let raw = p.as_str().trim().to_string();

                        if is_likely_type_only_param(&raw) {
                            ty = Some(raw);
                        } else {
                            name = Some(raw);
                        }
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
        param_type: ty,
        varargs,
    }
}

fn parse_method(pair: pest::iterators::Pair<Rule>) -> Method {
    fn parse_generic_param_list(pair: pest::iterators::Pair<Rule>) -> Vec<String> {
        pair.into_inner()
            .filter(|p| p.as_rule() == Rule::template_param)
            .map(|p| p.as_str().to_string())
            .collect()
    }

    let mut method = Method::default();
    let mut vis = None;
    let mut name = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::static_modifier | Rule::abstract_modifier | Rule::const_method_suffix => {
                method.modifiers.push(p.as_str().to_string())
            }
            Rule::class_visibility => vis = Some(p),
            Rule::method_name | Rule::identifier => name = Some(p.as_str().to_string()),
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
                        method.r#type = Some(return_type_inner.as_str().trim().to_string());
                    }
                }
            }
            Rule::generic_param_list => {
                method.generic_params.extend(parse_generic_param_list(p));
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

    fn walk<T>(pair: pest::iterators::Pair<Rule>, def: &mut T)
    where
        T: TypeDef,
    {
        match pair.as_rule() {
            Rule::named => {
                parse_named(pair, def.name_mut());
            }
            Rule::class_body => {
                for inner in pair.into_inner() {
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
            _ => {
                for inner in pair.into_inner() {
                    walk(inner, def);
                }
            }
        }
    }

    walk(pair, &mut def);

    def
}

fn parse_type_def(pair: pest::iterators::Pair<Rule>) -> Element {
    debug_assert_eq!(pair.as_rule(), Rule::type_def);

    fn find_type_kind(pair: pest::iterators::Pair<Rule>) -> Option<String> {
        if pair.as_rule() == Rule::type_kind {
            return Some(pair.as_str().to_string());
        }

        for inner in pair.into_inner() {
            if let Some(kind) = find_type_kind(inner) {
                return Some(kind);
            }
        }

        None
    }

    fn collect_extends_targets(pair: pest::iterators::Pair<Rule>) -> Vec<String> {
        fn walk(pair: pest::iterators::Pair<Rule>, targets: &mut Vec<String>) {
            match pair.as_rule() {
                Rule::extends_clause => {
                    for inner in pair.into_inner() {
                        if matches!(inner.as_rule(), Rule::extends_target | Rule::class_qualified_name)
                        {
                            targets.push(inner.as_str().to_string());
                        }
                    }
                }
                _ => {
                    for inner in pair.into_inner() {
                        walk(inner, targets);
                    }
                }
            }
        }

        let mut targets = Vec::new();
        walk(pair, &mut targets);
        targets
    }

    fn collect_implements_targets(pair: pest::iterators::Pair<Rule>) -> Vec<String> {
        fn walk(pair: pest::iterators::Pair<Rule>, targets: &mut Vec<String>) {
            match pair.as_rule() {
                Rule::implements_clause => {
                    for inner in pair.into_inner() {
                        if matches!(
                            inner.as_rule(),
                            Rule::implements_target | Rule::class_qualified_name
                        ) {
                            targets.push(inner.as_str().to_string());
                        }
                    }
                }
                _ => {
                    for inner in pair.into_inner() {
                        walk(inner, targets);
                    }
                }
            }
        }

        let mut targets = Vec::new();
        walk(pair, &mut targets);
        targets
    }

    fn collect_type_template_params(pair: pest::iterators::Pair<Rule>) -> Vec<String> {
        fn walk(pair: pest::iterators::Pair<Rule>, params: &mut Vec<String>) {
            match pair.as_rule() {
                Rule::type_generic_param_list => {
                    params.extend(
                        pair.into_inner()
                            .filter(|inner| inner.as_rule() == Rule::template_param)
                            .map(|inner| inner.as_str().to_string()),
                    );
                }
                Rule::class_body => {}
                _ => {
                    for inner in pair.into_inner() {
                        walk(inner, params);
                    }
                }
            }
        }

        let mut params = Vec::new();
        walk(pair, &mut params);
        params
    }

    fn parse_template_param_list_text(text: &str) -> Vec<String> {
        PlantUmlCommonParser::parse(Rule::type_generic_param_list, text)
            .ok()
            .and_then(|mut pairs| pairs.next())
            .map(|pair| {
                pair.into_inner()
                    .filter(|inner| inner.as_rule() == Rule::template_param)
                    .map(|inner| inner.as_str().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn infer_template_params_from_template_string(name: &str) -> Vec<String> {
        if !name.contains("<<template>>") {
            return Vec::new();
        }

        let candidate = name
            .rsplit("\\n")
            .next()
            .unwrap_or(name)
            .rsplit('\n')
            .next()
            .unwrap_or(name)
            .trim();

        let Some(start) = candidate.find('<') else {
            return Vec::new();
        };
        let Some(end) = candidate.rfind('>') else {
            return Vec::new();
        };

        if end <= start {
            return Vec::new();
        }

        parse_template_param_list_text(&candidate[start..=end])
    }

    fn resolve_type_template_params(explicit: Vec<String>, internal_name: &str) -> Vec<String> {
        if !explicit.is_empty() {
            explicit
        } else {
            infer_template_params_from_template_string(internal_name)
        }
    }

    let kind = find_type_kind(pair.clone()).expect("type_def must have type_kind");
    let explicit_template_params = collect_type_template_params(pair.clone());
    let extends_targets = collect_extends_targets(pair.clone());
    let implements_targets = collect_implements_targets(pair.clone());

    match kind.as_str() {
        "class" => {
            let mut def = parse_type_def_into::<ClassDef>(pair);
            def.template_params =
                resolve_type_template_params(explicit_template_params, &def.name.internal);
            def.extends = extends_targets;
            def.implements = implements_targets;
            Element::ClassDef(def)
        }
        "struct" => {
            let mut def = parse_type_def_into::<StructDef>(pair);
            def.template_params =
                resolve_type_template_params(explicit_template_params, &def.name.internal);
            Element::StructDef(def)
        }
        "interface" => {
            let mut def = parse_type_def_into::<InterfaceDef>(pair);
            def.template_params =
                resolve_type_template_params(explicit_template_params, &def.name.internal);
            def.extends = extends_targets;
            Element::InterfaceDef(def)
        }
        "object" => {
            let mut def = parse_type_def_into::<ObjectDef>(pair);
            def.template_params =
                resolve_type_template_params(explicit_template_params, &def.name.internal);
            Element::ObjectDef(def)
        }
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

fn visit_top_level<F>(pair: pest::iterators::Pair<Rule>, visitor: &mut F)
where
    F: FnMut(pest::iterators::Pair<Rule>),
{
    match pair.as_rule() {
        Rule::top_level | Rule::together_def => {
            for inner in pair.into_inner() {
                visit_top_level(inner, visitor);
            }
        }
        _ => visitor(pair),
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
                visit_top_level(inner, &mut |top_level_inner| match top_level_inner.as_rule() {
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
                    });
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
                visit_top_level(inner, &mut |t| match t.as_rule() {
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
                    });
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
        let normalized_content = normalize_multiline_member_signatures(content);

        // Log file content at trace level
        if matches!(log_level, LogLevel::Trace) {
            eprintln!(
                "{}:\n{}\n{}",
                path.display(),
                normalized_content.as_str(),
                "=".repeat(30)
            );
        }

        let mut uml_file = ClassUmlFile::default();

        match PlantUmlCommonParser::parse(Rule::class_start, normalized_content.as_str()) {
            Ok(mut pairs) => {
                let file_pair = pairs.next().unwrap();

                let inner = file_pair.into_inner();

                for pair in inner {
                    match pair.as_rule() {
                        Rule::top_level => {
                            visit_top_level(pair, &mut |inner_pair| match inner_pair.as_rule() {
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
                                });
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
                return Err(ClassError::Base(remap_syntax_error_to_original_source(
                    pest_to_syntax_error(
                        e,
                        path.as_ref().clone(),
                        normalized_content.as_str(),
                    ),
                    content,
                    &normalized_content,
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
        assert_eq!(param.param_type.as_deref(), Some("int"));
        assert!(param.varargs);
    }

    #[test]
    fn test_parse_param_name_only() {
        let input = "callable";
        let pair = PlantUmlCommonParser::parse(Rule::param, input)
            .unwrap()
            .next()
            .unwrap();

        let param = super::parse_param(pair);

        assert_eq!(param.name.as_deref(), Some("callable"));
        assert_eq!(param.param_type, None);
        assert!(!param.varargs);
    }

    #[test]
    fn test_parse_param_type_only_pascal_case() {
        let input = "InfrastructureContext";
        let pair = PlantUmlCommonParser::parse(Rule::param, input)
            .unwrap()
            .next()
            .unwrap();

        let param = super::parse_param(pair);

        assert_eq!(param.name, None);
        assert_eq!(param.param_type.as_deref(), Some("InfrastructureContext"));
        assert!(!param.varargs);
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
    fn test_parse_type_only_attribute() {
        let input = r#"@startuml
            class A {
                - std::mutex
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

        let ClassUmlTopLevel::Types(Element::ClassDef(class_def)) = &result.elements[0] else {
            panic!("expected class element");
        };

        assert_eq!(class_def.attributes.len(), 1);
        assert_eq!(class_def.attributes[0].name, "");
        assert_eq!(class_def.attributes[0].r#type.as_deref(), Some("std::mutex"));
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
