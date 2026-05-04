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
    TypeAlias, Visibility,
};
use crate::class_traits::{TypeDef, WritableName};
use crate::source_map::{
    normalize_multiline_member_signatures, remap_syntax_error_to_original_source, NormalizedContent,
};
use log::{debug, trace};
use parser_core::common_parser::{parse_arrow, PlantUmlCommonParser, Rule};
use parser_core::{format_parse_tree, pest_to_syntax_error, BaseParseError, DiagramParser};
use pest::Parser;
use puml_utils::LogLevel;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClassError {
    #[error(transparent)]
    Base(#[from] BaseParseError<Rule>),
    #[error("encountered using attribute while parsing a class attribute")]
    UnexpectedUsingAttribute,
    #[error("unexpected class member rule: {0}")]
    UnexpectedClassMember(String),
}

// Object definitions are ignored by the class parser, but their names must be
// tracked long enough to drop relationships that reference those ignored objects.
#[derive(Debug, Default)]
struct IgnoredObjectRegistry {
    ids: HashSet<String>,
    names: HashSet<String>,
}

impl IgnoredObjectRegistry {
    fn normalize_fqn(raw: &str) -> String {
        raw.replace("::", ".").trim_matches('.').to_string()
    }

    fn build_fqn(name: &str, parent: &Option<String>) -> String {
        let normalized_name = Self::normalize_fqn(name);

        match parent {
            Some(p) => {
                let normalized_parent = Self::normalize_fqn(p);

                if normalized_parent.is_empty() {
                    normalized_name
                } else if normalized_name.is_empty() {
                    normalized_parent
                } else {
                    format!("{}.{}", normalized_parent, normalized_name)
                }
            }
            None => normalized_name,
        }
    }

    fn register(&mut self, name: &Name, parent: &Option<String>) {
        self.ids.insert(Self::build_fqn(&name.internal, parent));
        self.names.insert(name.internal.clone());

        if let Some(alias) = &name.display {
            self.names.insert(alias.clone());
        }
    }

    fn merge(&mut self, other: Self) {
        self.ids.extend(other.ids);
        self.names.extend(other.names);
    }

    fn contains_reference(&self, name: &str, parent: &Option<String>) -> bool {
        let normalized = Self::normalize_fqn(name);

        self.ids.contains(&normalized)
            || self.ids.contains(&Self::build_fqn(name, parent))
            || self.names.contains(name)
    }

    fn filters_relationship(&self, relationship: &Relationship, parent: &Option<String>) -> bool {
        self.contains_reference(&relationship.left, parent)
            || self.contains_reference(&relationship.right, parent)
    }
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

fn parse_attribute(pair: pest::iterators::Pair<Rule>) -> Result<Attribute, ClassError> {
    let mut attr = Attribute::default();
    let mut vis = None;
    let mut name = None;
    let mut typ = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::static_modifier => attr.modifiers.push(p.as_str().to_string()),
            Rule::class_visibility => vis = Some(p),
            Rule::using_attribute => return Err(ClassError::UnexpectedUsingAttribute),
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
    Ok(attr)
}

fn parse_type_alias(pair: pest::iterators::Pair<Rule>) -> TypeAlias {
    let mut alias = None;
    let mut original_type = None;

    for p in pair.into_inner() {
        if p.as_rule() != Rule::using_attribute {
            continue;
        }

        for inner in p.into_inner() {
            match inner.as_rule() {
                Rule::identifier => alias = Some(inner.as_str().to_string()),
                Rule::type_name => original_type = Some(inner.as_str().trim().to_string()),
                _ => {}
            }
        }
    }

    TypeAlias {
        alias: alias.unwrap_or_default(),
        original_type: original_type.unwrap_or_default(),
    }
}

enum ParsedClassMember {
    Attribute(Attribute),
    TypeAlias(TypeAlias),
    Method(Method),
}

fn parse_class_member(pair: pest::iterators::Pair<Rule>) -> Result<ParsedClassMember, ClassError> {
    match pair.as_rule() {
        Rule::attribute => {
            let is_type_alias = pair
                .clone()
                .into_inner()
                .any(|inner| inner.as_rule() == Rule::using_attribute);

            if is_type_alias {
                Ok(ParsedClassMember::TypeAlias(parse_type_alias(pair)))
            } else {
                Ok(ParsedClassMember::Attribute(parse_attribute(pair)?))
            }
        }
        Rule::method => Ok(ParsedClassMember::Method(parse_method(pair))),
        _ => Err(ClassError::UnexpectedClassMember(format!(
            "{:?}",
            pair.as_rule()
        ))),
    }
}

fn parse_param(pair: pest::iterators::Pair<Rule>) -> Param {
    fn is_likely_type_only_param(raw: &str) -> bool {
        const PRIMITIVE_TYPES: &[&str] = &[
            "bool", "char", "short", "int", "long", "float", "double", "void", "size_t", "ssize_t",
            "uint8", "uint16", "uint32", "uint64", "int8", "int16", "int32", "int64", "auto",
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

        trimmed
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
    }

    let mut name: Option<String> = None;
    let mut ty: Option<String> = None;
    let mut varargs = false;

    // param -> param_named | param_cpp_named | param_unnamed
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

        Rule::param_cpp_named => {
            for p in inner.into_inner() {
                match p.as_rule() {
                    Rule::type_name => {
                        ty = Some(p.as_str().trim().to_string());
                    }
                    Rule::identifier => {
                        name = Some(p.as_str().to_string());
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

    fn ensure_abstract_modifier(method: &mut Method) {
        if !method
            .modifiers
            .iter()
            .any(|modifier| modifier == "{abstract}")
        {
            method.modifiers.push("{abstract}".to_string());
        }
    }

    let mut method = Method::default();
    let mut vis = None;
    let mut name = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::static_modifier
            | Rule::abstract_modifier
            | Rule::const_method_qualifier
            | Rule::noexcept_method_qualifier => method.modifiers.push(p.as_str().to_string()),
            Rule::pure_virtual_suffix => ensure_abstract_modifier(&mut method),
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
                    match return_type_inner.as_rule() {
                        Rule::static_modifier => {
                            method
                                .modifiers
                                .push(return_type_inner.as_str().to_string());
                        }
                        Rule::type_name => {
                            method.r#type = Some(return_type_inner.as_str().trim().to_string());
                        }
                        _ => {}
                    }
                }
            }
            Rule::generic_param_list => {
                method.template_parameters = Some(parse_generic_param_list(p));
            }
            _ => (),
        }
    }
    method.visibility = parse_visibility(vis);
    method.name = name.unwrap_or_default();

    method
}

fn parse_type_def_into<T>(pair: pest::iterators::Pair<Rule>) -> Result<T, ClassError>
where
    T: TypeDef + Default,
{
    let source_line = pair.as_span().start_pos().line_col().0 as u32;
    let mut def = T::default();
    *def.source_line_mut() = Some(source_line);

    fn walk<T>(pair: pest::iterators::Pair<Rule>, def: &mut T) -> Result<(), ClassError>
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
                            match parse_class_member(member)? {
                                ParsedClassMember::Attribute(attribute) => {
                                    def.attributes_mut().push(attribute)
                                }
                                ParsedClassMember::TypeAlias(type_alias) => {
                                    def.type_aliases_mut().push(type_alias)
                                }
                                ParsedClassMember::Method(method) => def.methods_mut().push(method),
                            }
                        }
                    }
                }
            }
            _ => {
                for inner in pair.into_inner() {
                    walk(inner, def)?;
                }
            }
        }

        Ok(())
    }

    walk(pair, &mut def)?;

    Ok(def)
}

fn parse_ignored_object_name(pair: pest::iterators::Pair<Rule>) -> Name {
    fn walk(pair: pest::iterators::Pair<Rule>, name: &mut Name) {
        match pair.as_rule() {
            Rule::named => parse_named(pair, name),
            _ => {
                for inner in pair.into_inner() {
                    walk(inner, name);
                }
            }
        }
    }

    let mut name = Name::default();
    walk(pair, &mut name);
    name
}

fn filter_relationships(
    relationships: Vec<Relationship>,
    ignored_objects: &IgnoredObjectRegistry,
    parent: &Option<String>,
) -> Vec<Relationship> {
    relationships
        .into_iter()
        .filter(|relationship| !ignored_objects.filters_relationship(relationship, parent))
        .collect()
}

fn original_start_line(
    pair: &pest::iterators::Pair<Rule>,
    normalized_content: &NormalizedContent,
) -> u32 {
    let (line, column) = pair.as_span().start_pos().line_col();
    normalized_content.map_position(line, column).0 as u32
}

fn parse_type_def(
    pair: pest::iterators::Pair<Rule>,
    normalized_content: &NormalizedContent,
) -> Result<Element, ClassError> {
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
                        if matches!(
                            inner.as_rule(),
                            Rule::extends_target | Rule::class_qualified_name
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

    fn collect_type_template_parameters(pair: pest::iterators::Pair<Rule>) -> Option<Vec<String>> {
        fn walk(pair: pest::iterators::Pair<Rule>, params: &mut Option<Vec<String>>) {
            match pair.as_rule() {
                Rule::type_generic_param_list => {
                    *params = Some(
                        pair.into_inner()
                            .filter(|inner| inner.as_rule() == Rule::template_param)
                            .map(|inner| inner.as_str().to_string())
                            .collect(),
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

        let mut params = None;
        walk(pair, &mut params);
        params
    }

    fn parse_template_parameter_list_text(text: &str) -> Option<Vec<String>> {
        let trimmed = text.trim();
        if trimmed.starts_with('<') && trimmed.ends_with('>') {
            let inner = trimmed[1..trimmed.len() - 1].trim();
            if inner.is_empty() {
                return Some(vec![]);
            }
        }

        PlantUmlCommonParser::parse(Rule::type_generic_param_list, text)
            .ok()
            .and_then(|mut pairs| pairs.next())
            .map(|pair| {
                pair.into_inner()
                    .filter(|inner| inner.as_rule() == Rule::template_param)
                    .map(|inner| inner.as_str().to_string())
                    .collect()
            })
    }

    fn infer_template_parameters_from_template_string(name: &str) -> Option<Vec<String>> {
        if !name.contains("<<template>>") {
            return None;
        }

        let candidate = name
            .rsplit("\\n")
            .next()
            .unwrap_or(name)
            .rsplit('\n')
            .next()
            .unwrap_or(name)
            .trim();

        let start = candidate.find('<')?;
        let end = candidate.rfind('>')?;

        if end <= start {
            return None;
        }

        parse_template_parameter_list_text(&candidate[start..=end])
    }

    fn infer_template_parameters_from_type_def_text(raw_type_def: &str) -> Option<Vec<String>> {
        let marker_index = raw_type_def.find("<<template>>")?;
        let template_label = raw_type_def[marker_index..]
            .split('"')
            .next()
            .unwrap_or_default();

        infer_template_parameters_from_template_string(template_label)
    }

    fn resolve_type_template_parameters(
        explicit: Option<Vec<String>>,
        name: &Name,
        raw_type_def: &str,
    ) -> Option<Vec<String>> {
        explicit.or_else(|| {
            name.display
                .as_deref()
                .and_then(infer_template_parameters_from_template_string)
                .or_else(|| infer_template_parameters_from_template_string(&name.internal))
                .or_else(|| infer_template_parameters_from_type_def_text(raw_type_def))
        })
    }

    let raw_type_def = pair.as_str().to_string();
    let source_line = original_start_line(&pair, normalized_content);
    let kind = find_type_kind(pair.clone()).expect("type_def must have type_kind");
    let explicit_template_parameters = collect_type_template_parameters(pair.clone());
    let extends_targets = collect_extends_targets(pair.clone());
    let implements_targets = collect_implements_targets(pair.clone());

    match kind.as_str() {
        "abstract class" => {
            let mut def = parse_type_def_into::<ClassDef>(pair)?;
            def.source_line = Some(source_line);
            def.is_abstract = true;
            def.template_parameters = resolve_type_template_parameters(
                explicit_template_parameters,
                &def.name,
                &raw_type_def,
            );
            def.extends = extends_targets;
            def.implements = implements_targets;
            Ok(Element::ClassDef(def))
        }
        "class" => {
            let mut def = parse_type_def_into::<ClassDef>(pair)?;
            def.source_line = Some(source_line);
            def.template_parameters = resolve_type_template_parameters(
                explicit_template_parameters,
                &def.name,
                &raw_type_def,
            );
            def.extends = extends_targets;
            def.implements = implements_targets;
            Ok(Element::ClassDef(def))
        }
        "struct" => {
            let mut def = parse_type_def_into::<StructDef>(pair)?;
            def.source_line = Some(source_line);
            def.template_parameters = resolve_type_template_parameters(
                explicit_template_parameters,
                &def.name,
                &raw_type_def,
            );
            Ok(Element::StructDef(def))
        }
        "interface" => {
            let mut def = parse_type_def_into::<InterfaceDef>(pair)?;
            def.source_line = Some(source_line);
            def.template_parameters = resolve_type_template_parameters(
                explicit_template_parameters,
                &def.name,
                &raw_type_def,
            );
            def.extends = extends_targets;
            Ok(Element::InterfaceDef(def))
        }
        _ => unreachable!("unknown type_kind: {}", kind),
    }
}

fn parse_enum_def(
    pair: pest::iterators::Pair<Rule>,
    normalized_content: &NormalizedContent,
) -> EnumDef {
    let mut enum_def = EnumDef {
        source_line: Some(original_start_line(&pair, normalized_content)),
        ..EnumDef::default()
    };

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

fn flatten_top_level(pair: pest::iterators::Pair<Rule>) -> Vec<pest::iterators::Pair<Rule>> {
    match pair.as_rule() {
        Rule::top_level | Rule::together_def => {
            pair.into_inner().flat_map(flatten_top_level).collect()
        }
        _ => vec![pair],
    }
}

fn parse_top_level_element(
    pair: pest::iterators::Pair<Rule>,
    normalized_content: &NormalizedContent,
    ignored_objects: &mut IgnoredObjectRegistry,
    relationships: &mut Vec<Relationship>,
) -> Result<Vec<ClassUmlTopLevel>, ClassError> {
    match pair.as_rule() {
        Rule::type_def => {
            let type_def = parse_type_def(pair, normalized_content)?;
            Ok(vec![ClassUmlTopLevel::Types(type_def)])
        }
        Rule::unsupported_object_def => {
            let ignored = parse_ignored_object_name(pair);
            ignored_objects.register(&ignored, &None);
            Ok(vec![])
        }
        Rule::enum_def => Ok(vec![ClassUmlTopLevel::Enum(parse_enum_def(
            pair,
            normalized_content,
        ))]),
        Rule::namespace_def => {
            let (namespace, nested_ignored_objects) = parse_namespace(pair, normalized_content)?;
            ignored_objects.merge(nested_ignored_objects);
            Ok(vec![ClassUmlTopLevel::Namespace(namespace)])
        }
        Rule::relationship => {
            relationships.push(parse_relationship(pair));
            Ok(vec![])
        }
        Rule::package_def => {
            let (package, nested_ignored_objects) = parse_package(pair, normalized_content)?;
            ignored_objects.merge(nested_ignored_objects);
            Ok(vec![ClassUmlTopLevel::Package(package)])
        }
        _ => Ok(vec![]),
    }
}

fn parse_namespace(
    pair: pest::iterators::Pair<Rule>,
    normalized_content: &NormalizedContent,
) -> Result<(Namespace, IgnoredObjectRegistry), ClassError> {
    let mut namespace = Namespace::default();
    let mut ignored_objects = IgnoredObjectRegistry::default();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::named => {
                parse_named(inner, &mut namespace.name);
            }
            Rule::top_level => {
                for top_level_inner in flatten_top_level(inner) {
                    match top_level_inner.as_rule() {
                        Rule::type_def => {
                            let mut type_def = parse_type_def(top_level_inner, normalized_content)?;
                            type_def.set_namespace(namespace.name.internal.clone());
                            namespace.types.push(type_def);
                        }
                        Rule::unsupported_object_def => {
                            let ignored = parse_ignored_object_name(top_level_inner);
                            ignored_objects
                                .register(&ignored, &Some(namespace.name.internal.clone()));
                        }
                        Rule::enum_def => {
                            let mut enum_def = Element::EnumDef(parse_enum_def(
                                top_level_inner,
                                normalized_content,
                            ));
                            enum_def.set_namespace(namespace.name.internal.clone());
                            namespace.types.push(enum_def);
                        }
                        Rule::namespace_def => {
                            let (nested_namespace, nested_ignored_objects) =
                                parse_namespace(top_level_inner, normalized_content)?;
                            ignored_objects.merge(nested_ignored_objects);
                            namespace.namespaces.push(nested_namespace);
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }
    }

    Ok((namespace, ignored_objects))
}

fn parse_package(
    pair: pest::iterators::Pair<Rule>,
    normalized_content: &NormalizedContent,
) -> Result<(Package, IgnoredObjectRegistry), ClassError> {
    let mut package = Package::default();
    let mut ignored_objects = IgnoredObjectRegistry::default();
    let mut relationships = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::named => {
                parse_named(inner, &mut package.name);
            }

            Rule::top_level => {
                for t in flatten_top_level(inner) {
                    match t.as_rule() {
                        Rule::type_def => {
                            let mut r#type = parse_type_def(t, normalized_content)?;
                            r#type.set_package(package.name.internal.clone());
                            package.types.push(r#type);
                        }
                        Rule::unsupported_object_def => {
                            let ignored = parse_ignored_object_name(t);
                            ignored_objects
                                .register(&ignored, &Some(package.name.internal.clone()));
                        }
                        Rule::enum_def => {
                            let mut enum_def =
                                Element::EnumDef(parse_enum_def(t, normalized_content));
                            enum_def.set_package(package.name.internal.clone());
                            package.types.push(enum_def);
                        }
                        Rule::relationship => {
                            relationships.push(parse_relationship(t));
                        }
                        Rule::package_def => {
                            let (subpackage, nested_ignored_objects) =
                                parse_package(t, normalized_content)?;
                            ignored_objects.merge(nested_ignored_objects);
                            package.packages.push(subpackage);
                        }
                        _ => (),
                    }
                }
            }
            _ => {}
        }
    }

    package.relationships = filter_relationships(
        relationships,
        &ignored_objects,
        &Some(package.name.internal.clone()),
    );

    Ok((package, ignored_objects))
}

fn parse_label(pair: pest::iterators::Pair<Rule>) -> String {
    pair.as_str().trim().to_string()
}

fn strip_wrapping_quotes(text: &str) -> String {
    if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
        text[1..text.len() - 1].to_string()
    } else {
        text.to_string()
    }
}

fn parse_relationship(pair: pest::iterators::Pair<Rule>) -> Relationship {
    let mut inner = pair.into_inner();

    let left = inner.next().unwrap().as_str().trim().to_string();

    let next = inner.next().unwrap();
    let (left_multiplicity, arrow_pair) = if next.as_rule() == Rule::relationship_multiplicity {
        (
            Some(strip_wrapping_quotes(next.as_str().trim())),
            inner.next().unwrap(),
        )
    } else {
        (None, next)
    };

    let arrow = parse_arrow(arrow_pair).unwrap_or_else(|_| Arrow::default());

    let next = inner.next().unwrap();
    let (right_multiplicity, right_pair) = if next.as_rule() == Rule::relationship_multiplicity {
        (
            Some(strip_wrapping_quotes(next.as_str().trim())),
            inner.next().unwrap(),
        )
    } else {
        (None, next)
    };

    let right = right_pair.as_str().trim().to_string();

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
        left_multiplicity,
        right_multiplicity,
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
            trace!(
                "{}:\n{}\n{}",
                path.display(),
                normalized_content.as_str(),
                "=".repeat(30)
            );
        }

        let mut uml_file = ClassUmlFile::default();
        let mut ignored_objects = IgnoredObjectRegistry::default();
        let mut relationships = Vec::new();

        match PlantUmlCommonParser::parse(Rule::class_start, normalized_content.as_str()) {
            Ok(pairs) => {
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

                let mut pairs = pairs;
                let file_pair = pairs.next().unwrap();

                let inner = file_pair.into_inner();

                for pair in inner {
                    match pair.as_rule() {
                        Rule::top_level => {
                            for inner_pair in flatten_top_level(pair) {
                                let mut elements = parse_top_level_element(
                                    inner_pair,
                                    &normalized_content,
                                    &mut ignored_objects,
                                    &mut relationships,
                                )?;
                                uml_file.elements.append(&mut elements);
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
                return Err(ClassError::Base(remap_syntax_error_to_original_source(
                    pest_to_syntax_error(e, path.as_ref().clone(), normalized_content.as_str()),
                    content,
                    &normalized_content,
                )));
            }
        };

        uml_file.relationships = filter_relationships(relationships, &ignored_objects, &None);

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
        assert_eq!(
            class_def.attributes[0].r#type.as_deref(),
            Some("std::mutex")
        );
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
        assert_eq!(rel.left_multiplicity, None);
        assert_eq!(rel.right_multiplicity, None);
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
