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
    Arrow, Attribute, ClassLikeDef, ClassLikeKind, ClassUmlFile, ClassUmlTopLevel,
    DeclarationCommon, EntityDef, EnumDef, EnumItem, EnumValue, Method, Name, Namespace, Package,
    Param, ParameterDirection, QualifiedTypeName, Relationship, TemplateParameter, TypeAlias,
    TypeArgument, TypeDefinition, TypeDescription, TypeExpr, Visibility,
};
use crate::source_map::{
    normalize_multiline_member_signatures, remap_syntax_error_to_original_source, NormalizedContent,
};
use log::{debug, trace, warn};
use parser_core::common_parser::{parse_arrow, PlantUmlCommonParser, Rule};
use parser_core::{
    find_note_alias, format_parse_tree, pest_to_syntax_error, BaseParseError, DiagramParser,
    ErrorLocation, IgnoredNoteRegistry,
};
use pest::Parser;
use puml_utils::LogLevel;
use source_location::SourceLocation;
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
    #[error("invalid parameter multiplicity [{value}] at {source_location}: {reason}")]
    InvalidParameterMultiplicity {
        value: String,
        reason: String,
        source_location: SourceLocation,
    },
    #[error("invalid return multiplicity [{value}] at {source_location}: {reason}")]
    InvalidReturnMultiplicity {
        value: String,
        reason: String,
        source_location: SourceLocation,
    },
}

impl ErrorLocation for ClassError {
    fn error_location(&self) -> Option<(usize, usize)> {
        match self {
            Self::Base(b) => b.error_location(),
            Self::InvalidParameterMultiplicity {
                source_location, ..
            }
            | Self::InvalidReturnMultiplicity {
                source_location, ..
            } => Some((source_location.line as usize, 1)),
            _ => None,
        }
    }
}

// Object definitions are ignored by the class parser, but their names must be
// tracked long enough to drop relationships that reference those ignored objects.
#[derive(Debug, Default)]
struct IgnoredObjectRegistry {
    ids: HashSet<String>,
    names: HashSet<String>,
}

impl IgnoredObjectRegistry {
    fn build_fqn(name: &str, parent: &Option<String>) -> String {
        let normalized_name = uid_utils::normalize(name);

        match parent {
            Some(p) => {
                let normalized_parent = uid_utils::normalize(p);

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

        if let Some(alias) = &name.alias {
            self.names.insert(alias.clone());
        }
    }

    fn merge(&mut self, other: Self) {
        self.ids.extend(other.ids);
        self.names.extend(other.names);
    }

    fn contains_reference(&self, name: &str, parent: &Option<String>) -> bool {
        let normalized = uid_utils::normalize(name);

        self.ids.contains(&normalized)
            || self.ids.contains(&Self::build_fqn(name, parent))
            || self.names.contains(name)
    }

    fn filters_relationship(&self, relationship: &Relationship, parent: &Option<String>) -> bool {
        self.contains_reference(&relationship.left, parent)
            || self.contains_reference(&relationship.right, parent)
    }
}

#[derive(Clone, Copy)]
enum TypeDefinitionScope {
    Namespace,
    Package,
}
impl TypeDefinitionScope {
    fn assign(self, definition: &mut TypeDefinition, name: &str) {
        match self {
            Self::Namespace => definition.declaration_mut().namespace = name.to_string(),
            Self::Package => definition.declaration_mut().package = name.to_string(),
        }
    }
}

struct ClassParseSession<'a> {
    normalized_content: &'a NormalizedContent,
    source_file: Rc<str>,
    ignored_objects: IgnoredObjectRegistry,
    ignored_notes: IgnoredNoteRegistry,
    relationships: Vec<Relationship>,
}

fn parse_type(pair: pest::iterators::Pair<Rule>) -> TypeDescription {
    TypeDescription {
        raw_text: pair.as_str().trim().to_string(),
        expression: parse_type_expr(pair),
    }
}

fn parse_type_expr(pair: pest::iterators::Pair<Rule>) -> Option<TypeExpr> {
    match pair.as_rule() {
        Rule::type_name => parse_type_expr(pair.into_inner().next()?),
        Rule::type_expr => {
            let raw_text = pair.as_str();
            let mut expression = None;
            let mut leading_const = false;
            let mut leading_volatile = false;
            for child in pair.into_inner() {
                match child.as_rule() {
                    Rule::const_qualifier => {
                        if leading_const {
                            warn!("Duplicate const qualifier in type expression {raw_text:?}; preserving raw text only");
                            return None;
                        }
                        leading_const = true;
                    }
                    Rule::volatile_qualifier => {
                        if leading_volatile {
                            warn!("Duplicate volatile qualifier in type expression {raw_text:?}; preserving raw text only");
                            return None;
                        }
                        leading_volatile = true;
                    }
                    Rule::builtin_type => {
                        expression = Some(TypeExpr::Builtin(
                            child
                                .as_str()
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" "),
                        ));
                    }
                    Rule::qualified_type => {
                        // Preserve decltype(...) as raw text to avoid mistaking it for a function type;
                        // add dedicated parsing later if needed.
                        if child.as_str() == "decltype" {
                            return None;
                        }
                        let normalized = child.as_str().replace("::", ".");
                        let name = normalized.strip_prefix('.').unwrap_or(&normalized);
                        if name.split('.').any(|segment| {
                            segment
                                .chars()
                                .next()
                                .is_some_and(|character| character.is_ascii_digit())
                        }) {
                            return None;
                        }
                        expression = Some(TypeExpr::NamedType(QualifiedTypeName {
                            segments: name.split('.').map(Into::into).collect(),
                            is_global: normalized.starts_with('.'),
                        }));
                    }
                    Rule::generic_type_args => {
                        let TypeExpr::NamedType(name) = expression.take()? else {
                            return None;
                        };
                        let arguments = parse_type_arguments(child)?;
                        expression = Some(TypeExpr::Template { name, arguments });
                    }
                    Rule::type_suffix
                    | Rule::function_type_suffix
                    | Rule::function_pointer_suffix => {
                        let mut inner = expression.take()?;
                        if leading_const {
                            inner = TypeExpr::Const(Box::new(inner));
                        }
                        if leading_volatile {
                            inner = TypeExpr::Volatile(Box::new(inner));
                        }
                        leading_const = false;
                        leading_volatile = false;
                        expression = Some(match child.as_rule() {
                            Rule::function_type_suffix => parse_function_type(inner, child, false)?,
                            Rule::function_pointer_suffix => {
                                parse_function_pointer(inner, child, raw_text)?
                            }
                            _ => parse_type_suffix(inner, child, raw_text)?,
                        });
                    }
                    Rule::function_type_noexcept => {
                        let TypeExpr::Function { is_noexcept, .. } = expression.as_mut()? else {
                            return None;
                        };
                        *is_noexcept = true;
                    }
                    _ => return None,
                }
            }
            let mut expression = expression?;
            if leading_const {
                expression = TypeExpr::Const(Box::new(expression));
            }
            if leading_volatile {
                expression = TypeExpr::Volatile(Box::new(expression));
            }
            Some(expression)
        }
        _ => None,
    }
}

fn parse_function_type(
    return_type: TypeExpr,
    parameters: pest::iterators::Pair<Rule>,
    is_noexcept: bool,
) -> Option<TypeExpr> {
    let mut parameter_types = Vec::new();
    let mut is_variadic = false;
    for child in parameters.into_inner() {
        match child.as_rule() {
            Rule::type_expr_list => {
                parameter_types = child
                    .into_inner()
                    .map(parse_type_expr)
                    .collect::<Option<_>>()?;
            }
            Rule::varargs => is_variadic = true,
            _ => return None,
        }
    }
    if matches!(parameter_types.as_slice(), [TypeExpr::Builtin(name)] if name == "void") {
        parameter_types.clear();
    }
    Some(TypeExpr::Function {
        return_type: Box::new(return_type),
        parameter_types,
        is_variadic,
        is_noexcept,
    })
}

fn parse_function_pointer(
    return_type: TypeExpr,
    suffix: pest::iterators::Pair<Rule>,
    raw_text: &str,
) -> Option<TypeExpr> {
    let mut children = suffix.into_inner();
    let declarator = children.next()?;
    let parameters = children.next()?;
    let expression = parse_function_type(return_type, parameters, children.next().is_some())?;
    parse_type_suffix(expression, declarator, raw_text)
}

// Parses template actual arguments: types recurse, while literal values retain their text.
// An empty list is valid; any unsupported argument makes the whole list return None.
fn parse_type_arguments(pair: pest::iterators::Pair<Rule>) -> Option<Vec<TypeArgument>> {
    pair.into_inner()
        .map(|argument| {
            let value = argument.into_inner().next()?;
            if value
                .as_str()
                .bytes()
                .all(|character| character.is_ascii_digit())
            {
                return Some(TypeArgument::Value(value.as_str().to_string()));
            }
            match value.as_rule() {
                Rule::template_value => {
                    Some(TypeArgument::Value(value.as_str().trim().to_string()))
                }
                Rule::type_expr => parse_type_expr(value).map(TypeArgument::Type),
                _ => None,
            }
        })
        .collect()
}

fn append_array_dimension(expression: TypeExpr, size: Option<String>) -> TypeExpr {
    match expression {
        TypeExpr::Array {
            element,
            size: outer_size,
        } => TypeExpr::Array {
            element: Box::new(append_array_dimension(*element, size)),
            size: outer_size,
        },
        element => TypeExpr::Array {
            element: Box::new(element),
            size,
        },
    }
}

// Wraps the existing type with pointer/reference qualifiers or an array suffix.
// Later array dimensions are nested inside earlier ones: T[3][8] has outer size 3.
fn parse_type_suffix(
    mut expression: TypeExpr,
    suffix: pest::iterators::Pair<Rule>,
    raw_text: &str,
) -> Option<TypeExpr> {
    let mut pointer_const = false;
    let mut pointer_volatile = false;
    for child in suffix.into_inner() {
        expression = match child.as_rule() {
            Rule::array_suffix => {
                let size = child
                    .into_inner()
                    .next()
                    .map(|size| size.as_str().to_string());
                append_array_dimension(expression, size)
            }
            Rule::pointer_suffix => TypeExpr::Pointer(Box::new(expression)),
            Rule::reference_suffix if child.as_str() == "&&" => {
                TypeExpr::RValueReference(Box::new(expression))
            }
            Rule::reference_suffix => TypeExpr::Reference(Box::new(expression)),
            Rule::const_qualifier => {
                if pointer_const {
                    warn!("Duplicate const qualifier on pointer in type expression {raw_text:?}; preserving raw text only");
                    return None;
                }
                pointer_const = true;
                TypeExpr::Const(Box::new(expression))
            }
            Rule::volatile_qualifier => {
                if pointer_volatile {
                    warn!("Duplicate volatile qualifier on pointer in type expression {raw_text:?}; preserving raw text only");
                    return None;
                }
                pointer_volatile = true;
                TypeExpr::Volatile(Box::new(expression))
            }
            _ => return None,
        };
    }
    Some(expression)
}

impl ClassParseSession<'_> {
    fn parse_template_parameters(
        pair: pest::iterators::Pair<Rule>,
    ) -> Option<Vec<TemplateParameter>> {
        fn parse_parameter(pair: pest::iterators::Pair<Rule>) -> Option<TemplateParameter> {
            match pair.as_rule() {
                Rule::template_param => parse_parameter(pair.into_inner().next().unwrap()),
                Rule::template_template_inner_param => {
                    parse_parameter(pair.into_inner().next().unwrap())
                }
                // A signature identifies a specialization argument list, not
                // template parameters. Discard metadata for the whole list.
                Rule::function_signature_template_param => None,
                Rule::legacy_type_template_param | Rule::explicit_type_template_param => {
                    let mut name = String::new();
                    let mut is_pack = false;
                    for inner in pair.into_inner() {
                        match inner.as_rule() {
                            Rule::identifier => name = inner.as_str().to_string(),
                            Rule::varargs => is_pack = true,
                            _ => {}
                        }
                    }
                    Some(TemplateParameter::Type { name, is_pack })
                }
                Rule::non_type_template_param => {
                    let mut name = String::new();
                    let mut value_type = String::new();
                    let mut is_pack = false;
                    for inner in pair.into_inner() {
                        match inner.as_rule() {
                            Rule::non_type_template_param_type => {
                                value_type = inner.as_str().trim().to_string()
                            }
                            Rule::identifier => name = inner.as_str().to_string(),
                            Rule::varargs => is_pack = true,
                            _ => {}
                        }
                    }
                    Some(TemplateParameter::NonType {
                        name,
                        value_type,
                        is_pack,
                    })
                }
                Rule::template_template_param => {
                    let mut name = String::new();
                    let mut parameters = Vec::new();
                    let mut is_pack = false;
                    for inner in pair.into_inner() {
                        match inner.as_rule() {
                            Rule::template_template_inner_param => {
                                parameters.push(parse_parameter(inner)?);
                            }
                            Rule::explicit_type_template_param => {
                                let tail = parse_parameter(inner)?;
                                let TemplateParameter::Type {
                                    name: parameter_name,
                                    is_pack: parameter_is_pack,
                                } = tail
                                else {
                                    unreachable!("template-template tail must be a type parameter");
                                };
                                name = parameter_name;
                                is_pack = parameter_is_pack;
                            }
                            _ => {}
                        }
                    }
                    Some(TemplateParameter::Template {
                        name,
                        parameters,
                        is_pack,
                    })
                }
                rule => unreachable!("unexpected template parameter rule: {rule:?}"),
            }
        }

        pair.into_inner()
            .filter(|inner| inner.as_rule() == Rule::template_param)
            .map(parse_parameter)
            .collect()
    }

    fn extract_stereotypes(pair: pest::iterators::Pair<Rule>) -> Vec<String> {
        pair.into_inner()
            .filter(|inner| inner.as_rule() == Rule::STEREOTYPE_NAME)
            .flat_map(|inner| inner.as_str().split(','))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect()
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

    /// A quoted alias (`X as "Label"`) is the name, the bare side the alias.
    fn parse_named(pair: pest::iterators::Pair<Rule>, name: &mut Name) {
        #[derive(Default)]
        struct Parts {
            internal: Option<String>,
            alias: Option<String>,
            alias_quoted: bool,
        }

        fn is_quoted(s: &str) -> bool {
            s.starts_with('"') && s.ends_with('"') && s.len() >= 2
        }

        fn strip_quotes(s: &str) -> String {
            if is_quoted(s) {
                s[1..s.len() - 1].to_string()
            } else {
                s.to_string()
            }
        }

        fn walk(pair: pest::iterators::Pair<Rule>, parts: &mut Parts) {
            match pair.as_rule() {
                Rule::internal_name => {
                    let raw = pair.as_str().to_string();
                    let saw_inner = pair.clone().into_inner().next().is_some();

                    for inner in pair.into_inner() {
                        walk(inner, parts);
                    }

                    if !saw_inner {
                        parts.internal = Some(strip_quotes(&raw));
                    }
                }
                Rule::STRING | Rule::qualified_type => {
                    if parts.internal.is_none() {
                        parts.internal = Some(strip_quotes(pair.as_str()));
                    }
                }
                Rule::alias_clause => {
                    let mut inner = pair.into_inner();
                    if let Some(target) = inner.next() {
                        parts.alias = Some(strip_quotes(target.as_str()));
                        parts.alias_quoted = is_quoted(target.as_str());
                    }
                }
                _ => {
                    for inner in pair.into_inner() {
                        walk(inner, parts);
                    }
                }
            }
        }

        let mut parts = Parts::default();
        walk(pair, &mut parts);

        if let Some(internal) = parts.internal {
            match parts.alias {
                Some(alias) if parts.alias_quoted => {
                    name.write_name(alias, Some(internal));
                }
                alias => name.write_name(&internal, alias.as_deref()),
            }
        }
    }

    fn parse_attribute(&self, pair: pest::iterators::Pair<Rule>) -> Result<Attribute, ClassError> {
        let mut attr = Attribute {
            source_location: self.original_source_location(&pair),
            ..Attribute::default()
        };
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
                            Rule::type_name => {
                                typ = Some(parse_type(inner));
                            }
                            Rule::static_modifier | Rule::constexpr_modifier => {
                                attr.modifiers.push(inner.as_str().to_string())
                            }
                            _ => {}
                        }
                    }
                }
                Rule::unnamed_attribute => {
                    for inner in p.into_inner() {
                        if inner.as_rule() == Rule::type_name {
                            typ = Some(parse_type(inner));
                        }
                    }
                }
                _ => {} // LCOV_EXCL_LINE
            }
        }

        attr.visibility = Self::parse_visibility(vis);
        attr.name = name.unwrap_or_default();
        attr.r#type = typ;
        Ok(attr)
    }

    fn parse_type_alias(&self, pair: pest::iterators::Pair<Rule>) -> TypeAlias {
        let mut alias = None;
        let mut original_type = None;
        let source_location = self.original_source_location(&pair);

        for p in pair.into_inner() {
            if p.as_rule() != Rule::using_attribute {
                continue;
            }

            for inner in p.into_inner() {
                match inner.as_rule() {
                    Rule::identifier => alias = Some(inner.as_str().to_string()),
                    Rule::type_name => {
                        original_type = Some(parse_type(inner));
                    }
                    _ => {}
                }
            }
        }

        TypeAlias {
            alias: alias.unwrap_or_default(),
            original_type: original_type.unwrap_or_default(),
            source_location,
        }
    }

    fn parse_class_member_into(
        &self,
        pair: pest::iterators::Pair<Rule>,
        def: &mut ClassLikeDef,
    ) -> Result<(), ClassError> {
        match pair.as_rule() {
            Rule::attribute => {
                let is_type_alias = pair
                    .clone()
                    .into_inner()
                    .any(|inner| inner.as_rule() == Rule::using_attribute);

                if is_type_alias {
                    def.members.type_aliases.push(self.parse_type_alias(pair));
                } else {
                    def.members.attributes.push(self.parse_attribute(pair)?);
                }
            }
            Rule::method => {
                def.members.methods.push(self.parse_method(pair)?);
            }
            _ => Err(ClassError::UnexpectedClassMember(format!(
                "{:?}",
                pair.as_rule()
            )))?,
        }

        Ok(())
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

            trimmed
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
        }

        let mut name: Option<String> = None;
        let mut ty: Option<TypeDescription> = None;
        let mut direction = None;
        let mut multiplicity = None;
        let is_pack_expansion = pair.as_rule() == Rule::typed_param_pack;

        // param -> param_named | param_cpp_named | param_unnamed
        // typed_param_pack -> param ~ varargs
        let inner = pair.into_inner().next().unwrap();
        let inner = if inner.as_rule() == Rule::param {
            inner.into_inner().next().unwrap()
        } else {
            inner
        };

        match inner.as_rule() {
            Rule::param_named => {
                for p in inner.into_inner() {
                    match p.as_rule() {
                        Rule::param_direction => {
                            direction = Some(match p.as_str() {
                                "in" => ParameterDirection::In,
                                "out" => ParameterDirection::Out,
                                "inout" => ParameterDirection::InOut,
                                _ => unreachable!(),
                            });
                        }
                        Rule::identifier => {
                            name = Some(p.as_str().to_string());
                        }
                        Rule::type_name => {
                            ty = Some(parse_type(p));
                        }
                        Rule::param_multiplicity => {
                            let raw = p.as_str();
                            multiplicity = Some(raw[1..raw.len() - 1].to_string());
                        }
                        _ => {}
                    }
                }
            }

            Rule::param_cpp_named => {
                for p in inner.into_inner() {
                    match p.as_rule() {
                        Rule::type_name => {
                            ty = Some(parse_type(p));
                        }
                        Rule::identifier => {
                            name = Some(p.as_str().to_string());
                        }
                        _ => {}
                    }
                }
            }

            Rule::param_unnamed => {
                for p in inner.into_inner() {
                    if p.as_rule() == Rule::type_name {
                        let parsed_type = parse_type(p);

                        if matches!(
                            &parsed_type.expression,
                            Some(TypeExpr::Builtin(_) | TypeExpr::Const(_) | TypeExpr::Volatile(_))
                        ) || is_likely_type_only_param(&parsed_type.raw_text)
                        {
                            ty = Some(parsed_type);
                        } else {
                            name = Some(parsed_type.raw_text);
                        }
                    }
                }
            }

            _ => unreachable!(),
        }

        Param {
            name,
            param_type: ty,
            direction,
            multiplicity,
            is_c_variadic: false,
            is_pack_expansion,
        }
    }

    fn validate_parameter_multiplicities(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<(), ClassError> {
        if matches!(
            pair.as_rule(),
            Rule::param_multiplicity | Rule::return_multiplicity
        ) {
            let is_return = pair.as_rule() == Rule::return_multiplicity;
            let raw = pair.as_str();
            let value = &raw[1..raw.len() - 1];
            if let Some((lower, upper)) = value.trim().split_once("..") {
                let invalid = |reason: String| {
                    let source_location = self.original_source_location(&pair);
                    if is_return {
                        ClassError::InvalidReturnMultiplicity {
                            value: value.to_string(),
                            reason,
                            source_location,
                        }
                    } else {
                        ClassError::InvalidParameterMultiplicity {
                            value: value.to_string(),
                            reason,
                            source_location,
                        }
                    }
                };
                let bound = |text: &str, label: &str| {
                    text.trim().parse::<u64>().map_err(|_| {
                        invalid(format!(
                            "{label} bound exceeds the supported u64 maximum ({})",
                            u64::MAX
                        ))
                    })
                };
                let lower = bound(lower, "lower")?;
                if upper.trim() != "*" && lower > bound(upper, "upper")? {
                    return Err(invalid(
                        "lower bound must not exceed upper bound".to_string(),
                    ));
                }
            }
        } else {
            for inner in pair.into_inner() {
                self.validate_parameter_multiplicities(inner)?;
            }
        }
        Ok(())
    }

    fn parse_method(&self, pair: pest::iterators::Pair<Rule>) -> Result<Method, ClassError> {
        self.validate_parameter_multiplicities(pair.clone())?;
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
        let mut conversion_target = None;
        method.source_location = self.original_source_location(&pair);

        for p in pair.into_inner() {
            match p.as_rule() {
                Rule::static_modifier
                | Rule::abstract_modifier
                | Rule::virtual_modifier
                | Rule::override_modifier
                | Rule::const_method_qualifier
                | Rule::noexcept_method_qualifier
                | Rule::trailing_override_qualifier
                | Rule::final_method_qualifier => method.modifiers.push(p.as_str().to_string()),
                Rule::friend_specifier => method.is_friend = true,
                Rule::pure_virtual_suffix => ensure_abstract_modifier(&mut method),
                Rule::class_visibility => vis = Some(p),
                Rule::method_name | Rule::identifier => {
                    name = Some(p.as_str().to_string());
                    conversion_target = p
                        .into_inner()
                        .find(|inner| inner.as_rule() == Rule::conversion_operator_name)
                        .and_then(|inner| {
                            inner
                                .into_inner()
                                .find(|target| target.as_rule() == Rule::conversion_target)
                        })
                        .map(|target| target.as_str().trim().to_string());
                }
                Rule::param_list => {
                    for param_pair in p.into_inner() {
                        match param_pair.as_rule() {
                            Rule::param | Rule::typed_param_pack => {
                                method.params.push(Self::parse_param(param_pair));
                            }
                            Rule::c_variadic_param => method.params.push(Param {
                                name: None,
                                param_type: None,
                                direction: None,
                                multiplicity: None,
                                is_c_variadic: true,
                                is_pack_expansion: false,
                            }),
                            _ => {}
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
                                method.r#type = Some(parse_type(return_type_inner));
                            }
                            Rule::return_multiplicity => {
                                let raw = return_type_inner.as_str();
                                method.return_multiplicity =
                                    Some(raw[1..raw.len() - 1].to_string());
                            }
                            _ => {}
                        }
                    }
                }
                Rule::generic_param_list => {
                    method.template_parameters = Self::parse_template_parameters(p);
                }
                _ => (),
            }
        }
        method.visibility = Self::parse_visibility(vis);
        method.name = name.unwrap_or_default();

        if let Some(target) = conversion_target {
            if !method.params.is_empty()
                || method.is_friend
                || method
                    .modifiers
                    .iter()
                    .any(|modifier| modifier.contains("static"))
            {
                return Err(ClassError::UnexpectedClassMember(
                    "conversion operators must be non-static members with no parameters".into(),
                ));
            }
            // Preserve explicit return spelling; validation checks type consistency.
            if method.r#type.is_none() {
                method.r#type = Some(target.clone().into());
            }
            method.name = format!("operator {target}");
            method.conversion_target = Some(target);
        }

        Ok(method)
    }

    fn walk_class_like_def_into(
        &self,
        pair: pest::iterators::Pair<Rule>,
        def: &mut ClassLikeDef,
    ) -> Result<(), ClassError> {
        match pair.as_rule() {
            Rule::named => {
                Self::parse_named(pair, &mut def.declaration.name);
            }
            Rule::stereotype => {
                def.declaration
                    .stereotypes
                    .extend(Self::extract_stereotypes(pair));
            }
            Rule::class_body => {
                for inner in pair.into_inner() {
                    // Intentionally visit only Rule::class_member; mock_method_invocation pairs
                    // are modeled as absent from the class diagram, not as real methods.
                    if let Rule::class_member = inner.as_rule() {
                        for member in inner.into_inner() {
                            self.parse_class_member_into(member, def)?;
                        }
                    }
                }
            }
            _ => {
                for inner in pair.into_inner() {
                    self.walk_class_like_def_into(inner, def)?;
                }
            }
        }

        Ok(())
    }

    fn parse_class_like_def(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ClassLikeDef, ClassError> {
        let mut def = ClassLikeDef::default();

        self.walk_class_like_def_into(pair, &mut def)?;

        Ok(def)
    }

    fn parse_ignored_object_name(pair: pest::iterators::Pair<Rule>) -> Name {
        fn walk(pair: pest::iterators::Pair<Rule>, name: &mut Name) {
            match pair.as_rule() {
                Rule::named => ClassParseSession::parse_named(pair, name),
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

    fn filter_visible_relationships(
        &self,
        relationships: Vec<Relationship>,
        ignored_objects: &IgnoredObjectRegistry,
        parent: &Option<String>,
    ) -> Vec<Relationship> {
        relationships
            .into_iter()
            .filter(|relationship| !ignored_objects.filters_relationship(relationship, parent))
            .filter(|relationship| {
                !self
                    .ignored_notes
                    .filters_endpoints(&relationship.left, &relationship.right)
            })
            .collect()
    }

    fn original_source_location(&self, pair: &pest::iterators::Pair<Rule>) -> SourceLocation {
        let (line, column) = pair.as_span().start_pos().line_col();
        let original_line = self.normalized_content.map_position(line, column).0 as u32;
        SourceLocation::new(self.source_file.clone(), original_line)
    }

    fn parse_type_def(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<TypeDefinition, ClassError> {
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
                            if inner.as_rule() == Rule::extends_target {
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
                            if inner.as_rule() == Rule::implements_target {
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

        fn collect_type_template_parameters(
            pair: pest::iterators::Pair<Rule>,
        ) -> Option<Vec<TemplateParameter>> {
            fn walk(
                pair: pest::iterators::Pair<Rule>,
                params: &mut Option<Vec<TemplateParameter>>,
            ) {
                match pair.as_rule() {
                    Rule::type_generic_param_list => {
                        *params = ClassParseSession::parse_template_parameters(pair);
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

        fn parse_template_parameter_list_text(
            text: &str,
            class_name: &str,
        ) -> Option<Vec<TemplateParameter>> {
            let trimmed = text.trim();
            if trimmed.starts_with('<') && trimmed.ends_with('>') {
                let inner = trimmed[1..trimmed.len() - 1].trim();
                if inner.is_empty() {
                    return Some(vec![]);
                }
            }

            let mut pairs = match PlantUmlCommonParser::parse(
                Rule::type_generic_param_list_exact,
                text,
            ) {
                Ok(pairs) => pairs,
                Err(error) => {
                    debug!(
                        "Cannot infer template parameters for class {class_name:?} from {text:?}: {error}"
                    );
                    return None;
                }
            };

            pairs
                .next()
                .and_then(|pair| {
                    pair.into_inner()
                        .find(|inner| inner.as_rule() == Rule::type_generic_param_list)
                })
                .and_then(ClassParseSession::parse_template_parameters)
        }

        fn infer_template_parameters_from_template_string(
            name: &str,
            class_name: &str,
        ) -> Option<Vec<TemplateParameter>> {
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

            parse_template_parameter_list_text(&candidate[start..=end], class_name)
        }

        fn infer_template_parameters_from_class_name(name: &str) -> Option<Vec<TemplateParameter>> {
            let candidate = name.trim();
            if !candidate.ends_with('>') {
                return None;
            }

            let start = candidate.find('<')?;
            let base_name = candidate[..start].trim();
            if base_name.is_empty() || base_name.contains('>') {
                return None;
            }

            let parameters = parse_template_parameter_list_text(&candidate[start..], name)?;
            // In a bare class name, unnamed non-pack non-type entries may be
            // concrete arguments. Do not infer metadata for an ambiguous list.
            if parameters.iter().any(|parameter| {
                matches!(
                    parameter,
                    TemplateParameter::NonType { name, is_pack: false, .. } if name.is_empty()
                )
            }) {
                return None;
            }

            Some(parameters)
        }

        fn infer_template_parameters_from_type_def_text(
            raw_type_def: &str,
            class_name: &str,
        ) -> Option<Vec<TemplateParameter>> {
            let marker_index = raw_type_def.find("<<template>>")?;
            let template_label = raw_type_def[marker_index..]
                .split('"')
                .next()
                .unwrap_or_default();

            infer_template_parameters_from_template_string(template_label, class_name)
        }

        fn resolve_type_template_parameters(
            explicit: Option<Vec<TemplateParameter>>,
            name: &Name,
            raw_type_def: &str,
        ) -> Option<Vec<TemplateParameter>> {
            explicit.or_else(|| {
                name.alias
                    .as_deref()
                    .and_then(|alias| {
                        infer_template_parameters_from_template_string(alias, &name.internal)
                    })
                    .or_else(|| {
                        infer_template_parameters_from_template_string(
                            &name.internal,
                            &name.internal,
                        )
                    })
                    .or_else(|| infer_template_parameters_from_class_name(&name.internal))
                    .or_else(|| {
                        infer_template_parameters_from_type_def_text(raw_type_def, &name.internal)
                    })
            })
        }

        let raw_type_def = pair.as_str().to_string();
        let source_location = self.original_source_location(&pair);
        let kind = find_type_kind(pair.clone())
            .expect("type_def must have type_kind")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let explicit_template_parameters = collect_type_template_parameters(pair.clone());
        let extends_targets = collect_extends_targets(pair.clone());
        let implements_targets = collect_implements_targets(pair.clone());

        if kind == "entity" {
            let def = self.parse_class_like_def(pair)?;
            return Ok(TypeDefinition::EntityDef(EntityDef {
                declaration: DeclarationCommon {
                    source_location,
                    ..def.declaration
                },
                methods: def.members.methods,
            }));
        }

        let mut def = self.parse_class_like_def(pair)?;
        def.declaration.source_location = source_location;
        def.template_parameters = resolve_type_template_parameters(
            explicit_template_parameters,
            &def.declaration.name,
            &raw_type_def,
        );
        def.kind = match kind.as_str() {
            "abstract class" | "class" => ClassLikeKind::Class {
                is_abstract: kind == "abstract class",
                extends: extends_targets,
                implements: implements_targets,
            },
            "struct" => ClassLikeKind::Struct,
            "interface" => ClassLikeKind::Interface {
                extends: extends_targets,
            },
            _ => unreachable!("unknown type_kind: {}", kind),
        };

        Ok(TypeDefinition::ClassLikeDef(def))
    }

    fn parse_enum_def(&self, pair: pest::iterators::Pair<Rule>) -> EnumDef {
        let mut enum_def = EnumDef {
            declaration: DeclarationCommon {
                source_location: self.original_source_location(&pair),
                ..DeclarationCommon::default()
            },
            ..EnumDef::default()
        };

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::named => {
                    // enum_def.name = inner.as_str().trim().to_string();
                    Self::parse_named(inner, &mut enum_def.declaration.name);
                }
                Rule::stereotype => {
                    enum_def
                        .declaration
                        .stereotypes
                        .extend(Self::extract_stereotypes(inner));
                }
                Rule::enum_body => {
                    enum_def.items = self.parse_enum_body(inner);
                }
                _ => (),
            }
        }

        enum_def
    }

    fn parse_enum_body(&self, pair: pest::iterators::Pair<Rule>) -> Vec<EnumItem> {
        pair.into_inner()
            .filter(|p| p.as_rule() == Rule::enum_item)
            .map(|p| self.parse_enum_item(p))
            .collect()
    }

    fn parse_enum_item(&self, pair: pest::iterators::Pair<Rule>) -> EnumItem {
        let mut item = EnumItem {
            source_location: self.original_source_location(&pair),
            ..EnumItem::default()
        };

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::identifier => {
                    item.name = inner.as_str().to_string();
                }
                Rule::enum_value => {
                    item.value = Some(Self::parse_enum_value(inner));
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
            Rule::top_level | Rule::together_def => pair
                .into_inner()
                .flat_map(Self::flatten_top_level)
                .collect(),
            _ => vec![pair],
        }
    }

    fn parse_top_level_element(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<ClassUmlTopLevel>, ClassError> {
        match pair.as_rule() {
            Rule::note_declaration => {
                if let Some(alias) = find_note_alias(pair) {
                    self.ignored_notes.register(alias);
                }
                Ok(vec![])
            }
            Rule::type_def => {
                let type_definition = self.parse_type_def(pair)?;
                Ok(vec![ClassUmlTopLevel::Type(type_definition)])
            }
            Rule::enum_def => Ok(vec![ClassUmlTopLevel::Type(TypeDefinition::EnumDef(
                self.parse_enum_def(pair),
            ))]),
            Rule::unsupported_object_def => {
                let ignored = Self::parse_ignored_object_name(pair);
                self.ignored_objects.register(&ignored, &None);
                Ok(vec![])
            }
            Rule::namespace_def => {
                let (namespace, nested_ignored_objects) = self.parse_namespace(pair)?;
                self.ignored_objects.merge(nested_ignored_objects);
                Ok(vec![ClassUmlTopLevel::Namespace(namespace)])
            }
            Rule::package_def => {
                let (package, nested_ignored_objects) = self.parse_package(pair)?;
                self.ignored_objects.merge(nested_ignored_objects);
                Ok(vec![ClassUmlTopLevel::Package(package)])
            }
            Rule::relationship => {
                let source_location = self.original_source_location(&pair);
                self.relationships
                    .push(parse_relationship(pair, source_location));
                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }

    fn parse_scoped_item(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
        scope_name: &str,
        scope: TypeDefinitionScope,
        ignored_objects: &mut IgnoredObjectRegistry,
        relationships: &mut Vec<Relationship>,
        definitions: &mut Vec<TypeDefinition>,
    ) -> Result<(), ClassError> {
        match pair.as_rule() {
            Rule::note_declaration => {
                if let Some(alias) = find_note_alias(pair) {
                    self.ignored_notes.register(alias);
                }
            }
            Rule::type_def => {
                let mut definition = self.parse_type_def(pair)?;
                scope.assign(&mut definition, scope_name);
                definitions.push(definition);
            }
            Rule::enum_def => {
                let mut definition = TypeDefinition::EnumDef(self.parse_enum_def(pair));
                scope.assign(&mut definition, scope_name);
                definitions.push(definition);
            }
            Rule::unsupported_object_def => {
                let ignored = Self::parse_ignored_object_name(pair);
                ignored_objects.register(&ignored, &Some(scope_name.to_string()));
            }
            Rule::relationship => {
                let source_location = self.original_source_location(&pair);
                relationships.push(parse_relationship(pair, source_location));
            }
            _ => {}
        }

        Ok(())
    }

    fn parse_namespace(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<(Namespace, IgnoredObjectRegistry), ClassError> {
        let mut namespace = Namespace {
            source_location: self.original_source_location(&pair),
            ..Namespace::default()
        };
        let mut ignored_objects = IgnoredObjectRegistry::default();
        let mut relationships = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::named => {
                    Self::parse_named(inner, &mut namespace.name);
                }
                Rule::top_level => {
                    for top_level_inner in Self::flatten_top_level(inner) {
                        match top_level_inner.as_rule() {
                            Rule::namespace_def => {
                                let (nested_namespace, nested_ignored_objects) =
                                    self.parse_namespace(top_level_inner)?;
                                ignored_objects.merge(nested_ignored_objects);
                                namespace.namespaces.push(nested_namespace);
                            }
                            _ => self.parse_scoped_item(
                                top_level_inner,
                                &namespace.name.internal,
                                TypeDefinitionScope::Namespace,
                                &mut ignored_objects,
                                &mut relationships,
                                &mut namespace.types,
                            )?,
                        }
                    }
                }
                _ => (),
            }
        }

        namespace.relationships = self.filter_visible_relationships(
            relationships,
            &ignored_objects,
            &Some(namespace.name.internal.clone()),
        );

        Ok((namespace, ignored_objects))
    }

    fn parse_package(
        &mut self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<(Package, IgnoredObjectRegistry), ClassError> {
        let mut package = Package {
            source_location: self.original_source_location(&pair),
            ..Package::default()
        };
        let mut ignored_objects = IgnoredObjectRegistry::default();
        let mut relationships = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::named => {
                    Self::parse_named(inner, &mut package.name);
                }

                Rule::top_level => {
                    for t in Self::flatten_top_level(inner) {
                        match t.as_rule() {
                            Rule::package_def => {
                                let (subpackage, nested_ignored_objects) = self.parse_package(t)?;
                                ignored_objects.merge(nested_ignored_objects);
                                package.packages.push(subpackage);
                            }
                            _ => self.parse_scoped_item(
                                t,
                                &package.name.internal,
                                TypeDefinitionScope::Package,
                                &mut ignored_objects,
                                &mut relationships,
                                &mut package.types,
                            )?,
                        }
                    }
                }
                _ => {}
            }
        }

        package.relationships = self.filter_visible_relationships(
            relationships,
            &ignored_objects,
            &Some(package.name.internal.clone()),
        );

        Ok((package, ignored_objects))
    }
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

fn parse_relationship(
    pair: pest::iterators::Pair<Rule>,
    source_location: SourceLocation,
) -> Relationship {
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
        source_location,
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
        let source_file: Rc<str> = path.as_ref().clone().to_string_lossy().to_string().into();
        let mut session = ClassParseSession {
            normalized_content: &normalized_content,
            source_file,
            ignored_objects: IgnoredObjectRegistry::default(),
            ignored_notes: IgnoredNoteRegistry::default(),
            relationships: Vec::new(),
        };

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
                            for inner_pair in ClassParseSession::flatten_top_level(pair) {
                                let mut elements = session.parse_top_level_element(inner_pair)?;
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

                let relationships = std::mem::take(&mut session.relationships);
                uml_file.relationships = session.filter_visible_relationships(
                    relationships,
                    &session.ignored_objects,
                    &None,
                );
            }
            Err(e) => {
                return Err(ClassError::Base(remap_syntax_error_to_original_source(
                    pest_to_syntax_error(e, path.as_ref().clone(), normalized_content.as_str()),
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
    fn test_parsed_type_preserves_raw_and_optional_structure() {
        for (input, expression) in [
            ("int", Some(TypeExpr::Builtin("int".into()))),
            (
                "void(int)",
                Some(TypeExpr::Function {
                    return_type: Box::new(TypeExpr::Builtin("void".into())),
                    parameter_types: vec![TypeExpr::Builtin("int".into())],
                    is_variadic: false,
                    is_noexcept: false,
                }),
            ),
            ("decltype(value)", None),
        ] {
            let pair = PlantUmlCommonParser::parse(Rule::type_name, input)
                .unwrap()
                .next()
                .unwrap();
            assert_eq!(pair.as_span().end(), input.len());
            assert_eq!(
                parse_type(pair),
                TypeDescription {
                    raw_text: input.into(),
                    expression,
                },
            );
        }
    }

    fn structured_type(input: &str) -> Option<TypeExpr> {
        let pair = PlantUmlCommonParser::parse(Rule::type_name, input)
            .unwrap()
            .next()
            .unwrap();
        assert_eq!(pair.as_span().end(), input.len(), "{input}");
        parse_type_expr(pair)
    }

    fn named_type(segments: &[&str]) -> TypeExpr {
        TypeExpr::NamedType(QualifiedTypeName {
            segments: segments.iter().map(|segment| (*segment).into()).collect(),
            is_global: false,
        })
    }

    #[test]
    fn test_structured_type_nested_templates() {
        assert_eq!(
            structured_type("std::map<int, std::vector<const vehicle::Payload*>>&"),
            Some(TypeExpr::Reference(Box::new(TypeExpr::Template {
                name: QualifiedTypeName {
                    segments: vec!["std".into(), "map".into()],
                    is_global: false,
                },
                arguments: vec![
                    TypeArgument::Type(TypeExpr::Builtin("int".into())),
                    TypeArgument::Type(TypeExpr::Template {
                        name: QualifiedTypeName {
                            segments: vec!["std".into(), "vector".into()],
                            is_global: false,
                        },
                        arguments: vec![TypeArgument::Type(TypeExpr::Pointer(Box::new(
                            TypeExpr::Const(Box::new(named_type(&["vehicle", "Payload"]))),
                        )))],
                    }),
                ],
            })))
        );
    }

    #[test]
    fn test_structured_type_qualified_names() {
        for input in [
            "::vehicle::Payload",
            "::vehicle.Payload",
            ".vehicle.Payload",
            ".vehicle::Payload",
        ] {
            assert_eq!(
                structured_type(input),
                Some(TypeExpr::NamedType(QualifiedTypeName {
                    segments: vec!["vehicle".into(), "Payload".into()],
                    is_global: true,
                })),
                "{input}",
            );
        }
        for input in ["vehicle::Payload", "vehicle.Payload"] {
            assert_eq!(
                structured_type(input),
                Some(named_type(&["vehicle", "Payload"])),
                "{input}",
            );
        }
        assert_eq!(structured_type("Payload"), Some(named_type(&["Payload"])));
        assert_eq!(structured_type("constant"), Some(named_type(&["constant"])));
    }

    #[test]
    fn test_structured_type_preserves_const_position_and_reference_kind() {
        let payload = named_type(&["Payload"]);
        for (input, expected) in [
            (
                "const Payload*",
                TypeExpr::Pointer(Box::new(TypeExpr::Const(Box::new(payload.clone())))),
            ),
            (
                "Payload* const",
                TypeExpr::Const(Box::new(TypeExpr::Pointer(Box::new(payload.clone())))),
            ),
            (
                "Payload&&",
                TypeExpr::RValueReference(Box::new(payload.clone())),
            ),
            (
                "Payload**",
                TypeExpr::Pointer(Box::new(TypeExpr::Pointer(Box::new(payload.clone())))),
            ),
            ("const Payload", TypeExpr::Const(Box::new(payload))),
        ] {
            assert_eq!(structured_type(input), Some(expected), "{input}");
        }
    }

    #[test]
    fn test_structured_type_value_and_empty_template_arguments() {
        for (input, arguments) in [
            ("Buffer<8>", vec![TypeArgument::Value("8".into())]),
            ("Buffer<>", vec![]),
        ] {
            assert_eq!(
                structured_type(input),
                Some(TypeExpr::Template {
                    name: QualifiedTypeName {
                        segments: vec!["Buffer".into()],
                        is_global: false,
                    },
                    arguments,
                })
            );
        }
    }

    #[test]
    fn test_structured_type_cv_qualifiers() {
        for input in ["const Payload&", "Payload const&"] {
            assert_eq!(
                structured_type(input),
                Some(TypeExpr::Reference(Box::new(TypeExpr::Const(Box::new(
                    named_type(&["Payload"]),
                ))))),
                "{input}",
            );
        }
        for input in ["volatile int", "int volatile"] {
            assert_eq!(
                structured_type(input),
                Some(TypeExpr::Volatile(Box::new(TypeExpr::Builtin(
                    "int".into()
                )))),
                "{input}",
            );
        }
        let qualified_int = TypeExpr::Volatile(Box::new(TypeExpr::Const(Box::new(
            TypeExpr::Builtin("int".into()),
        ))));
        let qualified_pointer = TypeExpr::Volatile(Box::new(TypeExpr::Const(Box::new(
            TypeExpr::Pointer(Box::new(qualified_int)),
        ))));
        assert_eq!(
            structured_type("const volatile int* const volatile"),
            Some(qualified_pointer),
        );
        let const_int = TypeExpr::Const(Box::new(TypeExpr::Builtin("int".into())));
        let const_pointer = TypeExpr::Const(Box::new(TypeExpr::Pointer(Box::new(const_int))));
        assert_eq!(
            structured_type("const int* const* const"),
            Some(TypeExpr::Const(Box::new(TypeExpr::Pointer(Box::new(
                const_pointer,
            ))))),
        );
    }

    #[test]
    fn test_structured_type_duplicate_cv_qualifiers_preserve_raw_only() {
        for input in [
            "const const Payload",
            "volatile volatile int",
            "const Payload const&",
            "volatile int volatile*",
            "int* const const",
            "int* volatile volatile",
            "int* const volatile const",
            "int* const* volatile volatile",
            "Buffer<const const int>",
        ] {
            let pair = PlantUmlCommonParser::parse(Rule::type_name, input)
                .unwrap()
                .next()
                .unwrap();
            assert_eq!(pair.as_span().end(), input.len(), "{input}");
            assert_eq!(
                parse_type(pair),
                TypeDescription {
                    raw_text: input.into(),
                    expression: None,
                },
                "{input}",
            );
        }
    }

    #[test]
    fn test_structured_template_literal_arguments() {
        for value in ["8", "8U", "8u", "8UL", "8LLU", "8ull", "true", "false"] {
            assert_eq!(
                structured_type(&format!("Value<{value}>")),
                Some(TypeExpr::Template {
                    name: QualifiedTypeName {
                        segments: vec!["Value".into()],
                        is_global: false,
                    },
                    arguments: vec![TypeArgument::Value(value.into())],
                }),
                "{value}",
            );
        }
        assert_eq!(
            structured_type("Value<true_type>"),
            Some(TypeExpr::Template {
                name: QualifiedTypeName {
                    segments: vec!["Value".into()],
                    is_global: false,
                },
                arguments: vec![TypeArgument::Type(named_type(&["true_type"]))],
            }),
        );
    }

    #[test]
    fn test_cv_parameters_preserve_name_and_type() {
        for (parameters, expected_name, expected_type) in [
            ("volatile int", None, "volatile int"),
            ("volatile\tint", None, "volatile\tint"),
            ("volatile int entry", Some("entry"), "volatile int"),
            ("entry : volatile int", Some("entry"), "volatile int"),
            ("Payload const&", None, "Payload const&"),
            ("Payload const& entry", Some("entry"), "Payload const&"),
            ("entry : Payload const&", Some("entry"), "Payload const&"),
            ("payload const", None, "payload const"),
            ("payload\tconst", None, "payload\tconst"),
            ("int* volatile", None, "int* volatile"),
        ] {
            let result = parse_single_class_member_file(&format!("+ Set({parameters})")).unwrap();
            let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class)) = &result.elements[0]
            else {
                panic!("expected a class");
            };
            let method = &class.members.methods[0];
            assert_eq!(method.params.len(), 1, "{parameters}");
            let parameter = &method.params[0];
            assert_eq!(parameter.name.as_deref(), expected_name, "{parameters}");
            assert_eq!(
                parameter.param_type.as_ref().map(|ty| ty.raw_text.as_str()),
                Some(expected_type),
                "{parameters}",
            );
            assert_eq!(
                parameter
                    .param_type
                    .as_ref()
                    .and_then(|ty| ty.expression.clone()),
                structured_type(expected_type),
                "{parameters}",
            );
        }
    }

    #[test]
    fn test_template_literal_arguments_in_members() {
        let result = parse_single_class_member_file(
            "value : std::array<int, 8U>\nusing Flag = std::integral_constant<bool, true>\n+ Set(entry : std::array<int, 8U>) : std::integral_constant<bool, true>",
        )
        .unwrap();
        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class)) = &result.elements[0]
        else {
            panic!("expected a class element");
        };
        let array = "std::array<int, 8U>";
        let flag = "std::integral_constant<bool, true>";
        assert_eq!(
            class.members.attributes[0]
                .r#type
                .as_ref()
                .map(|ty| ty.raw_text.as_str()),
            Some(array)
        );
        assert_eq!(
            class.members.attributes[0]
                .r#type
                .as_ref()
                .and_then(|ty| ty.expression.clone()),
            structured_type(array)
        );
        assert_eq!(class.members.type_aliases[0].original_type.raw_text, flag);
        assert_eq!(
            class.members.type_aliases[0].original_type.expression,
            structured_type(flag)
        );
        let method = &class.members.methods[0];
        assert_eq!(method.params[0].name.as_deref(), Some("entry"));
        assert_eq!(
            method.params[0]
                .param_type
                .as_ref()
                .map(|ty| ty.raw_text.as_str()),
            Some(array)
        );
        assert_eq!(
            method.params[0]
                .param_type
                .as_ref()
                .and_then(|ty| ty.expression.clone()),
            structured_type(array)
        );
        assert_eq!(
            method.r#type.as_ref().map(|ty| ty.raw_text.as_str()),
            Some(flag)
        );
        assert_eq!(
            method.r#type.as_ref().and_then(|ty| ty.expression.clone()),
            structured_type(flag)
        );
    }

    #[test]
    fn test_structured_type_arrays() {
        let array = |element, size: Option<&str>| TypeExpr::Array {
            element: Box::new(element),
            size: size.map(str::to_string),
        };
        for (input, expected) in [
            ("Payload[8]", array(named_type(&["Payload"]), Some("8"))),
            ("Payload[8U]", array(named_type(&["Payload"]), Some("8U"))),
            (
                "Payload[8ull]",
                array(named_type(&["Payload"]), Some("8ull")),
            ),
            ("Payload[]", array(named_type(&["Payload"]), None)),
            ("Payload[N]", array(named_type(&["Payload"]), Some("N"))),
            (
                "Payload[3][8]",
                array(array(named_type(&["Payload"]), Some("8")), Some("3")),
            ),
            (
                "Payload[][8]",
                array(array(named_type(&["Payload"]), Some("8")), None),
            ),
            (
                "Payload*[8]",
                array(
                    TypeExpr::Pointer(Box::new(named_type(&["Payload"]))),
                    Some("8"),
                ),
            ),
            (
                "const Payload[8]",
                array(
                    TypeExpr::Const(Box::new(named_type(&["Payload"]))),
                    Some("8"),
                ),
            ),
            ("int[8]", array(TypeExpr::Builtin("int".into()), Some("8"))),
            (
                "Buffer<Payload[8]>",
                TypeExpr::Template {
                    name: QualifiedTypeName {
                        segments: vec!["Buffer".into()],
                        is_global: false,
                    },
                    arguments: vec![TypeArgument::Type(array(
                        named_type(&["Payload"]),
                        Some("8"),
                    ))],
                },
            ),
        ] {
            assert_eq!(structured_type(input), Some(expected), "{input}");
        }
        for suffix in ["[-1]", "[+8]", "[N + 1]"] {
            assert!(
                PlantUmlCommonParser::parse(Rule::array_suffix, suffix).is_err(),
                "{suffix}"
            );
        }
    }

    #[test]
    fn test_structured_type_function_pointers() {
        let function =
            |return_type, parameter_types, is_variadic, is_noexcept| TypeExpr::Function {
                return_type: Box::new(return_type),
                parameter_types,
                is_variadic,
                is_noexcept,
            };
        let pointer = |inner| TypeExpr::Pointer(Box::new(inner));
        let builtin = |name: &str| TypeExpr::Builtin(name.into());
        for (input, expected) in [
            (
                "void (*)()",
                pointer(function(builtin("void"), vec![], false, false)),
            ),
            (
                "void (*)(void)",
                pointer(function(builtin("void"), vec![], false, false)),
            ),
            (
                "int (*)(Payload&, double)",
                pointer(function(
                    builtin("int"),
                    vec![
                        TypeExpr::Reference(Box::new(named_type(&["Payload"]))),
                        builtin("double"),
                    ],
                    false,
                    false,
                )),
            ),
            (
                "const Payload* (*)(int)",
                pointer(function(
                    pointer(TypeExpr::Const(Box::new(named_type(&["Payload"])))),
                    vec![builtin("int")],
                    false,
                    false,
                )),
            ),
            (
                "void (* const)(int)",
                TypeExpr::Const(Box::new(pointer(function(
                    builtin("void"),
                    vec![builtin("int")],
                    false,
                    false,
                )))),
            ),
            (
                "void (* volatile)(int)",
                TypeExpr::Volatile(Box::new(pointer(function(
                    builtin("void"),
                    vec![builtin("int")],
                    false,
                    false,
                )))),
            ),
            (
                "void (*)(int, ...)",
                pointer(function(builtin("void"), vec![builtin("int")], true, false)),
            ),
            (
                "void (*)(...)",
                pointer(function(builtin("void"), vec![], true, false)),
            ),
            (
                "void (*)() noexcept",
                pointer(function(builtin("void"), vec![], false, true)),
            ),
            (
                "Wrapper<void (*)(int)>",
                TypeExpr::Template {
                    name: QualifiedTypeName {
                        segments: vec!["Wrapper".into()],
                        is_global: false,
                    },
                    arguments: vec![TypeArgument::Type(pointer(function(
                        builtin("void"),
                        vec![builtin("int")],
                        false,
                        false,
                    )))],
                },
            ),
            (
                "void (*)(void (*)(int))",
                pointer(function(
                    builtin("void"),
                    vec![pointer(function(
                        builtin("void"),
                        vec![builtin("int")],
                        false,
                        false,
                    ))],
                    false,
                    false,
                )),
            ),
        ] {
            assert_eq!(structured_type(input), Some(expected), "{input}");
        }
        for input in ["void (* const const)(int)", "void (*)(const const Payload)"] {
            assert_eq!(structured_type(input), None, "{input}");
        }
    }

    #[test]
    fn test_structured_type_functions() {
        let builtin = |name: &str| TypeExpr::Builtin(name.into());
        let function =
            |return_type, parameter_types, is_variadic, is_noexcept| TypeExpr::Function {
                return_type: Box::new(return_type),
                parameter_types,
                is_variadic,
                is_noexcept,
            };
        for (input, expected) in [
            ("void()", function(builtin("void"), vec![], false, false)),
            (
                "void(void)",
                function(builtin("void"), vec![], false, false),
            ),
            (
                "void(int)",
                function(builtin("void"), vec![builtin("int")], false, false),
            ),
            (
                "int(Payload&, double)",
                function(
                    builtin("int"),
                    vec![
                        TypeExpr::Reference(Box::new(named_type(&["Payload"]))),
                        builtin("double"),
                    ],
                    false,
                    false,
                ),
            ),
            (
                "const Payload*(int)",
                function(
                    TypeExpr::Pointer(Box::new(TypeExpr::Const(Box::new(named_type(&[
                        "Payload",
                    ]))))),
                    vec![builtin("int")],
                    false,
                    false,
                ),
            ),
            (
                "Payload&(int)",
                function(
                    TypeExpr::Reference(Box::new(named_type(&["Payload"]))),
                    vec![builtin("int")],
                    false,
                    false,
                ),
            ),
            (
                "void(int, ...)",
                function(builtin("void"), vec![builtin("int")], true, false),
            ),
            ("void(...)", function(builtin("void"), vec![], true, false)),
            (
                "void(int) noexcept",
                function(builtin("void"), vec![builtin("int")], false, true),
            ),
            (
                "std::function<void(int)>",
                TypeExpr::Template {
                    name: QualifiedTypeName {
                        segments: vec!["std".into(), "function".into()],
                        is_global: false,
                    },
                    arguments: vec![TypeArgument::Type(function(
                        builtin("void"),
                        vec![builtin("int")],
                        false,
                        false,
                    ))],
                },
            ),
            (
                "void(void (*)(int))",
                function(
                    builtin("void"),
                    vec![TypeExpr::Pointer(Box::new(function(
                        builtin("void"),
                        vec![builtin("int")],
                        false,
                        false,
                    )))],
                    false,
                    false,
                ),
            ),
        ] {
            assert_eq!(structured_type(input), Some(expected), "{input}");
        }
    }

    #[test]
    fn test_structured_type_unsupported_shapes_are_not_partially_represented() {
        for input in [
            "[]Payload",
            "void(const const Payload)",
            "{Payload}",
            "std::function<void(const const Payload)>",
            "decltype(value)",
            "const decltype(value)",
            "std::function<decltype(value)>",
            "Value<0xGG>",
        ] {
            assert_eq!(structured_type(input), None, "{input}");
        }
    }

    #[test]
    fn test_structured_types_preserve_member_strings() {
        let result = parse_single_class_member_file(
            "value : std::map<int, Payload*>\nusing Buffer = std::map<int, Payload*>\n+ Store(data : std::map<int, Payload*>&, Payload&&) : ::vehicle::Payload*",
        )
        .unwrap();
        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class_def)) = &result.elements[0]
        else {
            panic!("expected class element");
        };
        let attribute = &class_def.members.attributes[0];
        assert_eq!(
            attribute.r#type.as_ref().map(|ty| ty.raw_text.as_str()),
            Some("std::map<int, Payload*>")
        );
        assert_eq!(
            attribute
                .r#type
                .as_ref()
                .and_then(|ty| ty.expression.clone()),
            structured_type("std::map<int, Payload*>"),
        );
        let alias = &class_def.members.type_aliases[0];
        assert_eq!(alias.original_type.raw_text, "std::map<int, Payload*>");
        assert_eq!(
            alias.original_type.expression,
            attribute
                .r#type
                .as_ref()
                .and_then(|ty| ty.expression.clone())
        );
        let method = &class_def.members.methods[0];
        assert_eq!(
            method.r#type.as_ref().map(|ty| ty.raw_text.as_str()),
            Some("::vehicle::Payload*")
        );
        assert_eq!(
            method.r#type.as_ref().and_then(|ty| ty.expression.clone()),
            structured_type("::vehicle::Payload*")
        );
        assert_eq!(method.params[0].name.as_deref(), Some("data"));
        assert_eq!(
            method.params[0]
                .param_type
                .as_ref()
                .map(|ty| ty.raw_text.as_str()),
            Some("std::map<int, Payload*>&"),
        );
        assert_eq!(
            method.params[0]
                .param_type
                .as_ref()
                .and_then(|ty| ty.expression.clone()),
            structured_type("std::map<int, Payload*>&"),
        );
        assert_eq!(
            method.params[1]
                .param_type
                .as_ref()
                .map(|ty| ty.raw_text.as_str()),
            Some("Payload&&")
        );
        assert_eq!(
            method.params[1]
                .param_type
                .as_ref()
                .and_then(|ty| ty.expression.clone()),
            structured_type("Payload&&")
        );
    }

    #[test]
    fn test_array_member_types_preserve_strings_and_structure() {
        let result = parse_single_class_member_file(
            "values : Payload[8U]\nusing Buffer = Payload[8U]\n+ Store(data : Payload[8U]) : Payload[8U]",
        )
        .unwrap();
        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class_def)) = &result.elements[0]
        else {
            panic!("expected class element");
        };
        assert_eq!(
            class_def.members.attributes[0]
                .r#type
                .as_ref()
                .map(|ty| ty.raw_text.as_str()),
            Some("Payload[8U]"),
        );
        let expected = structured_type("Payload[8U]");
        assert!(expected.is_some());
        assert_eq!(
            class_def.members.attributes[0]
                .r#type
                .as_ref()
                .and_then(|ty| ty.expression.clone()),
            expected
        );
        assert_eq!(
            class_def.members.type_aliases[0].original_type.raw_text,
            "Payload[8U]"
        );
        assert_eq!(
            class_def.members.type_aliases[0].original_type.expression,
            expected
        );
        let method = &class_def.members.methods[0];
        assert_eq!(
            method.r#type.as_ref().map(|ty| ty.raw_text.as_str()),
            Some("Payload[8U]")
        );
        assert_eq!(
            method.r#type.as_ref().and_then(|ty| ty.expression.clone()),
            expected
        );
        assert_eq!(
            method.params[0]
                .param_type
                .as_ref()
                .map(|ty| ty.raw_text.as_str()),
            Some("Payload[8U]")
        );
        assert_eq!(
            method.params[0]
                .param_type
                .as_ref()
                .and_then(|ty| ty.expression.clone()),
            expected
        );
    }

    fn make_test_session(normalized_content: &NormalizedContent) -> ClassParseSession<'_> {
        ClassParseSession {
            normalized_content,
            source_file: Rc::from("test.puml"),
            ignored_objects: IgnoredObjectRegistry::default(),
            ignored_notes: IgnoredNoteRegistry::default(),
            relationships: Vec::new(),
        }
    }

    fn parse_single_class_member_file(member: &str) -> Result<ClassUmlFile, ClassError> {
        let input = format!("@startuml\nclass A {{\n    {member}\n}}\n@enduml\n");
        let mut parser = PumlClassParser;
        parser.parse_file(
            &std::rc::Rc::new(std::path::PathBuf::from("test.puml")),
            &input,
            LogLevel::Info,
        )
    }

    fn parse_class_diagram_body(body: &str) -> Result<ClassUmlFile, ClassError> {
        let input = format!("@startuml\n{body}\n@enduml\n");
        PumlClassParser.parse_file(
            &std::rc::Rc::new(std::path::PathBuf::from("test.puml")),
            &input,
            LogLevel::Info,
        )
    }

    #[test]
    fn test_parse_visibility_none() {
        let vis = super::ClassParseSession::parse_visibility(None);
        assert_eq!(vis, Visibility::Public);
    }

    #[test]
    fn test_parse_visibility_unknown_symbol() {
        let pair = PlantUmlCommonParser::parse(Rule::identifier, "abc")
            .unwrap()
            .next()
            .unwrap();

        let vis = super::ClassParseSession::parse_visibility(Some(pair));

        assert_eq!(vis, Visibility::Public);
    }

    #[test]
    fn test_parse_typed_param_pack() {
        let input = "int...";
        let pair = PlantUmlCommonParser::parse(Rule::typed_param_pack, input)
            .unwrap()
            .next()
            .unwrap();

        let param = super::ClassParseSession::parse_param(pair);

        assert_eq!(param.name, None);
        assert_eq!(
            param.param_type.as_ref().map(|ty| ty.raw_text.as_str()),
            Some("int")
        );
        assert!(param.is_pack_expansion);
    }

    #[test]
    fn test_parse_param_name_only() {
        let input = "callable";
        let pair = PlantUmlCommonParser::parse(Rule::param, input)
            .unwrap()
            .next()
            .unwrap();

        let param = super::ClassParseSession::parse_param(pair);

        assert_eq!(param.name.as_deref(), Some("callable"));
        assert_eq!(param.param_type, None);
        assert_eq!(
            param
                .param_type
                .as_ref()
                .and_then(|ty| ty.expression.clone()),
            None
        );
        assert!(!param.is_pack_expansion);
    }

    #[test]
    fn test_parse_param_type_only_pascal_case() {
        let input = "InfrastructureContext";
        let pair = PlantUmlCommonParser::parse(Rule::param, input)
            .unwrap()
            .next()
            .unwrap();

        let param = super::ClassParseSession::parse_param(pair);

        assert_eq!(param.name, None);
        assert_eq!(
            param.param_type.as_ref().map(|ty| ty.raw_text.as_str()),
            Some("InfrastructureContext")
        );
        assert!(!param.is_pack_expansion);
    }

    #[test]
    fn test_parse_param_pointer_trailing_const() {
        for (input, expected_type, expected_name) in [
            ("void* const", "void* const", None),
            ("void* const ptr", "void* const", Some("ptr")),
            ("int* const result", "int* const", Some("result")),
        ] {
            let pair = PlantUmlCommonParser::parse(Rule::param, input)
                .unwrap()
                .next()
                .unwrap();

            let param = super::ClassParseSession::parse_param(pair);

            assert_eq!(
                param.param_type.as_ref().map(|ty| ty.raw_text.as_str()),
                Some(expected_type),
                "{input}"
            );
            assert_eq!(param.name.as_deref(), expected_name, "{input}");
        }
    }

    #[test]
    fn test_parse_param_const_prefixed_name_not_treated_as_const() {
        // `constant` must be read as a name, not a `const` qualifier plus `ant`.
        let pair = PlantUmlCommonParser::parse(Rule::param, "void* constant")
            .unwrap()
            .next()
            .unwrap();

        let param = super::ClassParseSession::parse_param(pair);

        assert_eq!(
            param.param_type.as_ref().map(|ty| ty.raw_text.as_str()),
            Some("void*")
        );
        assert_eq!(param.name.as_deref(), Some("constant"));
    }

    #[test]
    fn test_parse_param_const_keyword_is_not_a_name() {
        let result = parse_single_class_member_file("+ f(int const)").unwrap();
        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class_def)) = &result.elements[0]
        else {
            panic!("expected class element");
        };
        let parameter = &class_def.members.methods[0].params[0];
        assert_eq!(parameter.name, None);
        assert_eq!(
            parameter.param_type.as_ref().map(|ty| ty.raw_text.as_str()),
            Some("int const")
        );
        assert_eq!(
            parameter
                .param_type
                .as_ref()
                .and_then(|ty| ty.expression.clone()),
            Some(TypeExpr::Const(Box::new(TypeExpr::Builtin("int".into())))),
        );
    }

    #[test]
    fn test_parse_method_with_const_pointer_and_const_value_params() {
        let result = parse_single_class_member_file(
            "+ do_deallocate(void* const p, const std::size_t bytes, const std::size_t alignment) override",
        )
        .unwrap();

        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class_def)) = &result.elements[0]
        else {
            panic!("expected class element");
        };

        let method = &class_def.members.methods[0];
        let params: Vec<_> = method
            .params
            .iter()
            .map(|param| {
                (
                    param.name.as_deref(),
                    param.param_type.as_ref().map(|ty| ty.raw_text.as_str()),
                )
            })
            .collect();
        assert_eq!(
            params,
            vec![
                (Some("p"), Some("void* const")),
                (Some("bytes"), Some("const std::size_t")),
                (Some("alignment"), Some("const std::size_t")),
            ]
        );
        assert_eq!(method.modifiers, vec!["override"]);
    }

    #[test]
    fn test_parse_const_pointer_return_type_is_not_a_const_method() {
        for (member, return_type, modifiers) in [
            ("+ Get() : Foo* const", "Foo* const", vec![]),
            (
                "+ Get() : Foo* const <<const>>",
                "Foo* const",
                vec!["<<const>>"],
            ),
            ("+ Get() const : Foo* const", "Foo* const", vec!["const"]),
            ("+ Get() const : Foo*", "Foo*", vec!["const"]),
            ("+ Get() : Foo* <<const>>", "Foo*", vec!["<<const>>"]),
        ] {
            let result = parse_single_class_member_file(member).expect(member);
            let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class_def)) =
                &result.elements[0]
            else {
                panic!("expected class element");
            };

            let method = &class_def.members.methods[0];
            assert_eq!(
                method.r#type.as_ref().map(|ty| ty.raw_text.as_str()),
                Some(return_type),
                "{member}"
            );
            assert_eq!(method.modifiers, modifiers, "{member}");
        }
    }

    #[test]
    fn test_parse_bare_const_after_return_type_qualifies_type() {
        for (member, return_type, modifiers) in [
            ("+ Get() : bool const", "bool const", vec![]),
            (
                "+ Get() : std::string const noexcept",
                "std::string const",
                vec!["noexcept"],
            ),
        ] {
            let result = parse_single_class_member_file(member).expect(member);
            let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class_def)) =
                &result.elements[0]
            else {
                panic!("expected class element");
            };
            let method = &class_def.members.methods[0];
            assert_eq!(
                method.r#type.as_ref().map(|ty| ty.raw_text.as_str()),
                Some(return_type),
                "{member}"
            );
            assert_eq!(
                method.r#type.as_ref().and_then(|ty| ty.expression.clone()),
                structured_type(return_type),
                "{member}"
            );
            assert_eq!(method.modifiers, modifiers, "{member}");
        }
    }

    #[test]
    fn test_parse_pointer_trailing_const_in_attribute_and_alias() {
        let result =
            parse_single_class_member_file("- p : int* const\n    +using P = int* const").unwrap();

        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class_def)) = &result.elements[0]
        else {
            panic!("expected class element");
        };

        assert_eq!(
            class_def.members.attributes[0]
                .r#type
                .as_ref()
                .map(|ty| ty.raw_text.as_str()),
            Some("int* const")
        );
        assert_eq!(
            class_def.members.type_aliases[0].original_type.raw_text,
            "int* const"
        );
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

        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class_def)) = &result.elements[0]
        else {
            panic!("expected class element");
        };

        assert_eq!(class_def.members.attributes.len(), 1);
        assert_eq!(class_def.members.attributes[0].name, "");
        assert_eq!(
            class_def.members.attributes[0]
                .r#type
                .as_ref()
                .map(|ty| ty.raw_text.as_str()),
            Some("std::mutex")
        );
        assert_eq!(class_def.members.attributes[0].source_location.line, 3);
    }

    #[test]
    fn test_static_attribute_modifier_before_and_after_visibility() {
        for modifier in ["static", "{static}", "<<static>>"] {
            for (symbol, visibility) in [
                ("-", Visibility::Private),
                ("+", Visibility::Public),
                ("#", Visibility::Protected),
            ] {
                for member in [
                    format!("{modifier} {symbol} mock_ : A*"),
                    format!("{symbol} {modifier} mock_ : A*"),
                ] {
                    let result = parse_single_class_member_file(&member).unwrap();
                    let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) =
                        &result.elements[0]
                    else {
                        panic!("expected class element");
                    };
                    assert!(def.members.methods.is_empty(), "{member}");
                    assert_eq!(def.members.attributes.len(), 1, "{member}");
                    let attr = &def.members.attributes[0];
                    assert_eq!(attr.visibility, visibility, "{member}");
                    assert_eq!(attr.name, "mock_", "{member}");
                    assert_eq!(
                        attr.r#type.as_ref().map(|ty| ty.raw_text.as_str()),
                        Some("A*"),
                        "{member}"
                    );
                    assert_eq!(attr.modifiers, vec![modifier.to_string()], "{member}");
                }
            }
        }
    }

    #[test]
    fn test_class_body_separator_requires_matching_delimiters_on_one_line() {
        for separator in [
            ".. Accessors --\n",
            "== Accessors __\n",
            "-- Accessors\n",
            ".. Accessors\n..\n",
            "== Accessors == trailing\n",
        ] {
            assert!(
                PlantUmlCommonParser::parse(Rule::class_body_separator, separator).is_err(),
                "{separator}"
            );
        }
    }

    #[test]
    fn test_parameter_direction_requires_a_valid_uml_named_parameter() {
        for parameter in ["input x : T", "in out x : T", "out :", "inout x :"] {
            let member = format!("+f({parameter}) : void");
            assert!(parse_single_class_member_file(&member).is_err(), "{member}");
        }
    }

    #[test]
    fn keyword_operators_do_not_store_conversion_metadata() {
        for (member, name, return_type, parameter_count) in [
            ("+operator delete() : void", "operator delete", "void", 0),
            (
                "+operator co_await() : Awaiter",
                "operator co_await",
                "Awaiter",
                0,
            ),
            (
                "+operator new(std::size_t n) : void*",
                "operator new",
                "void*",
                1,
            ),
            (
                "+static operator new[](std::size_t n) : void*",
                "operator new[]",
                "void*",
                1,
            ),
            (
                "+operator delete[](void* p) : void",
                "operator delete[]",
                "void",
                1,
            ),
        ] {
            let result = parse_single_class_member_file(member).expect(member);
            let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &result.elements[0]
            else {
                panic!("expected class element");
            };
            let method = &def.members.methods[0];
            assert_eq!(method.name, name, "{member}");
            assert_eq!(method.conversion_target, None, "{member}");
            assert_eq!(
                method.r#type.as_ref().map(|ty| ty.raw_text.as_str()),
                Some(return_type),
                "{member}"
            );
            assert_eq!(method.params.len(), parameter_count, "{member}");
        }
    }

    #[test]
    fn conversion_target_declarators_store_explicit_metadata() {
        for target in [
            "bool",
            "ns::Value<T>",
            "const T*",
            "T**&",
            "T* const&&",
            "new_type",
            "delete_type",
            "co_await_type",
        ] {
            let member = format!("+operator {target}()");
            let result = parse_single_class_member_file(&member).unwrap();
            let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &result.elements[0]
            else {
                panic!("expected class element");
            };
            assert_eq!(
                def.members.methods[0].conversion_target.as_deref(),
                Some(target)
            );
            assert_eq!(
                def.members.methods[0]
                    .r#type
                    .as_ref()
                    .map(|ty| ty.raw_text.as_str()),
                Some(target)
            );
        }
        for target in ["T&*", "T&&&", "T&&*", "T& &"] {
            assert!(
                parse_single_class_member_file(&format!("+operator {target}()")).is_err(),
                "{target}"
            );
        }
        for member in ["+operator()() : int", "+operatorX() : bool", "+f() : int"] {
            let result = parse_single_class_member_file(member).unwrap();
            let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &result.elements[0]
            else {
                panic!("expected class element");
            };
            assert_eq!(def.members.methods[0].conversion_target, None);
        }
    }

    #[test]
    fn test_parameter_multiplicity_rejects_invalid_inputs() {
        for value in [
            "-1..2", "1..-2", "*..1", "0..", "..1", "..*", "1...2", "1..2..3", "0..1.5", "0..N",
            "**",
        ] {
            assert!(
                parse_single_class_member_file(&format!("+f(x : T[{value}]) : void")).is_err(),
                "{value}"
            );
        }
        for member in [
            "+f(T[0..1] x) : void",
            "+f(T[*]) : void",
            "+x : T[0..1]",
            "+f(x : T[0..1][*]) : void",
        ] {
            assert!(parse_single_class_member_file(member).is_err(), "{member}");
        }

        for (value, reason) in [
            ("2..1", "lower bound must not exceed upper bound"),
            ("18446744073709551616..*", "lower bound exceeds"),
            ("0..18446744073709551616", "upper bound exceeds"),
        ] {
            let error =
                parse_single_class_member_file(&format!("+f(x : T[{value}]) : void")).unwrap_err();
            assert!(matches!(
                error,
                ClassError::InvalidParameterMultiplicity { .. }
            ));
            assert!(error.to_string().contains(reason), "{error}");
            assert!(error.to_string().contains(value), "{error}");
            assert_eq!(error.error_location(), Some((3, 1)));
        }
    }

    #[test]
    fn return_multiplicity_rejects_invalid_ranges() {
        for value in [
            "-1..2", "1..-2", "*..1", "0..", "..1", "..*", "1...2", "1..2..3", "0..1.5", "0..N",
            "**",
        ] {
            assert!(
                parse_single_class_member_file(&format!("+f() : T[{value}]")).is_err(),
                "{value}"
            );
        }
        for member in ["+f() : T[0..1][*]", "+f() : T[*][0..1]"] {
            assert!(parse_single_class_member_file(member).is_err(), "{member}");
        }
        for (value, reason) in [
            ("2..1", "lower bound must not exceed upper bound"),
            ("18446744073709551616..*", "lower bound exceeds"),
            ("0..18446744073709551616", "upper bound exceeds"),
        ] {
            let error = parse_single_class_member_file(&format!("+f() : T[{value}]")).unwrap_err();
            assert!(matches!(
                error,
                ClassError::InvalidReturnMultiplicity { .. }
            ));
            assert!(error.to_string().contains(reason), "{error}");
            assert!(error.to_string().contains(value), "{error}");
            assert_eq!(error.error_location(), Some((3, 1)));
        }
    }

    #[test]
    fn test_parse_empty_template_args_in_types() {
        let result = parse_single_class_member_file(
            "+Run(const safecpp::Scope<>& scope, std::vector<Foo<>> items): Bar<>",
        )
        .unwrap();

        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class_def)) = &result.elements[0]
        else {
            panic!("expected class element");
        };

        let method = &class_def.members.methods[0];
        assert_eq!(
            method.params[0]
                .param_type
                .as_ref()
                .map(|ty| ty.raw_text.as_str()),
            Some("const safecpp::Scope<>&")
        );
        assert_eq!(
            method.params[1]
                .param_type
                .as_ref()
                .map(|ty| ty.raw_text.as_str()),
            Some("std::vector<Foo<>>")
        );
        assert_eq!(
            method.r#type.as_ref().map(|ty| ty.raw_text.as_str()),
            Some("Bar<>")
        );
    }

    #[test]
    fn test_parse_empty_template_args_in_attribute() {
        let result = parse_single_class_member_file("+scope : safecpp::Scope<>").unwrap();

        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class_def)) = &result.elements[0]
        else {
            panic!("expected class element");
        };

        assert_eq!(
            class_def.members.attributes[0]
                .r#type
                .as_ref()
                .map(|ty| ty.raw_text.as_str()),
            Some("safecpp::Scope<>")
        );
    }

    #[test]
    fn test_function_signature_class_arguments_do_not_infer_template_parameters() {
        for signature in [
            "ReturnType()",
            "ReturnType(ArgType)",
            "ReturnType(ArgTypes...)",
            "ReturnType(A, Bs...)",
        ] {
            for arguments in [
                signature.to_string(),
                format!("{signature}, Capacity, Alignment"),
                format!("Capacity, {signature}, Alignment"),
                format!("Capacity, Alignment, {signature}"),
            ] {
                for declaration in [
                    format!("class move_only_function<{arguments}>"),
                    format!("class \"move_only_function<{arguments}>\" as move_only_function"),
                    format!("class move_only_function as \"move_only_function<{arguments}>\""),
                    format!("class \"<<template>>\\n<{arguments}>\" as move_only_function"),
                    format!("class move_only_function as \"<<template>>\\n<{arguments}>\""),
                ] {
                    let file = parse_class_diagram_body(&declaration).expect(&declaration);
                    let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) =
                        &file.elements[0]
                    else {
                        panic!("expected a class");
                    };

                    assert!(def.template_parameters.is_none(), "{declaration}");
                }
            }
        }
    }

    #[test]
    fn test_function_signature_method_arguments_do_not_infer_template_parameters() {
        for signature in ["R()", "R(A)", "R(Args...)", "R(A, Bs...)"] {
            for arguments in [
                signature.to_string(),
                format!("{signature}, T, U"),
                format!("T, {signature}, U"),
                format!("T, U, {signature}"),
            ] {
                for member in [
                    format!("+Run<{arguments}>()"),
                    format!("+Run()<{arguments}>"),
                ] {
                    let file = parse_single_class_member_file(&member).expect(&member);
                    let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) =
                        &file.elements[0]
                    else {
                        panic!("expected a class");
                    };

                    assert_eq!(def.members.methods.len(), 1, "{member}");
                    assert!(
                        def.members.methods[0].template_parameters.is_none(),
                        "{member}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_quoted_class_arguments_do_not_infer_template_parameters() {
        for name in [
            "vector<int>",
            "Map<std::string, int>",
            "Foo<std::vector>",
            "Foo<int, T, U>",
            "Foo<T, int, U>",
            "Foo<T, U, int>",
        ] {
            for declaration in [
                format!("class \"{name}\""),
                format!("class \"{name}\" as Alias"),
                format!("class Alias as \"{name}\""),
            ] {
                let file = parse_class_diagram_body(&declaration).expect(&declaration);
                let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &file.elements[0]
                else {
                    panic!("expected a class");
                };

                assert_eq!(def.declaration.name.internal, name, "{declaration}");
                assert!(def.template_parameters.is_none(), "{declaration}");
            }
        }
    }

    #[test]
    fn test_quoted_template_names_require_complete_parameter_lists() {
        for name in [
            "Foo<A>::Bar<B>",
            "Foo<A><B>",
            "Foo<R (*)(A...)>",
            "Foo<T = int>",
            "<<template>>\\n<A>::Bar<B>",
        ] {
            let declaration = format!("class \"{name}\"");
            let file = parse_class_diagram_body(&declaration).expect(&declaration);
            let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &file.elements[0]
            else {
                panic!("expected a class");
            };

            assert!(def.template_parameters.is_none(), "{declaration}");
        }
    }

    #[test]
    fn test_quoted_template_names_preserve_valid_and_empty_parameter_lists() {
        for (name, expected) in [
            (
                "Foo<A>",
                vec![TemplateParameter::Type {
                    name: "A".to_string(),
                    is_pack: false,
                }],
            ),
            (
                "<<template>>\\n<A>",
                vec![TemplateParameter::Type {
                    name: "A".to_string(),
                    is_pack: false,
                }],
            ),
            ("Foo<>", vec![]),
            ("Foo< >", vec![]),
        ] {
            let declaration = format!("class \"{name}\"");
            let file = parse_class_diagram_body(&declaration).expect(&declaration);
            let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &file.elements[0]
            else {
                panic!("expected a class");
            };

            assert_eq!(
                def.template_parameters.as_deref(),
                Some(expected.as_slice()),
                "{declaration}"
            );
        }
    }

    #[test]
    fn test_parse_template_parameter_keyword_prefixes() {
        for name in [
            "charT",
            "int32_t",
            "char16_type",
            "boolName",
            "wchar_type",
            "char8_type",
            "char32_type",
            "integer",
            "shortName",
            "longName",
            "unsignedName",
            "signedName",
            "className",
            "typenameName",
            "templateName",
            "int_value",
            "class_name",
            "typename2",
        ] {
            for (parameters, expected) in [
                (
                    format!("{name}, traits"),
                    vec![
                        TemplateParameter::Type {
                            name: name.to_string(),
                            is_pack: false,
                        },
                        TemplateParameter::Type {
                            name: "traits".to_string(),
                            is_pack: false,
                        },
                    ],
                ),
                (
                    format!("{name} N"),
                    vec![TemplateParameter::NonType {
                        name: "N".to_string(),
                        value_type: name.to_string(),
                        is_pack: false,
                    }],
                ),
            ] {
                for quoted in [false, true] {
                    let internal_name = format!("Box<{parameters}>");
                    let declaration = if quoted {
                        format!("class \"{internal_name}\"")
                    } else {
                        format!("class {internal_name}")
                    };
                    let file = parse_class_diagram_body(&declaration).expect(&declaration);
                    let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) =
                        &file.elements[0]
                    else {
                        panic!("expected a class");
                    };

                    assert_eq!(
                        def.template_parameters.as_deref(),
                        Some(expected.as_slice()),
                        "{declaration}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_reject_template_parameter_packs_after_explicit_names() {
        for parameters in [
            "typename T...",
            "class T...",
            "typename... T...",
            "int N...",
            "int... Ns...",
            "template<typename> class C...",
            "template<typename> class... Cs...",
            "template<typename T...> class C",
            "template<int N...> class C",
        ] {
            let list = format!("<{parameters}>");
            assert!(
                PlantUmlCommonParser::parse(Rule::type_generic_param_list, &list).is_err(),
                "must reject {list}"
            );
            let declaration = format!("class Box<{parameters}>");
            assert!(
                parse_class_diagram_body(&declaration).is_err(),
                "{declaration}"
            );

            // Quoted display names remain valid PlantUML, but must not yield
            // template metadata for an invalid C++ parameter declaration.
            let declaration = format!("class \"Box<{parameters}>\"");
            let file = parse_class_diagram_body(&declaration).expect(&declaration);
            let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &file.elements[0]
            else {
                panic!("expected a class");
            };
            assert!(def.template_parameters.is_none(), "{declaration}");
        }
    }

    #[test]
    fn test_reject_invalid_non_type_template_pointer_reference_order() {
        for value_type in ["int", "Widget", "const Widget", "std::size_t", "Box<int>"] {
            for suffix in ["&*", "&&*", "&&&", "& &", "&& &&", "*&*", "**&&&"] {
                for name in ["", " N"] {
                    let parameters = format!("{value_type}{suffix}{name}");
                    let list = format!("<{parameters}>");
                    assert!(
                        PlantUmlCommonParser::parse(Rule::type_generic_param_list, &list).is_err(),
                        "must reject {list}"
                    );
                    let declaration = format!("class Box<{parameters}>");
                    assert!(
                        parse_class_diagram_body(&declaration).is_err(),
                        "{declaration}"
                    );
                    let declaration = format!("class \"Box<{parameters}>\"");
                    let file = parse_class_diagram_body(&declaration).expect(&declaration);
                    let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) =
                        &file.elements[0]
                    else {
                        panic!("expected a class");
                    };
                    assert!(def.template_parameters.is_none(), "{declaration}");
                    let member = format!("+Run<{parameters}>()");
                    // PlantUML may retain malformed method text as an attribute,
                    // but it must not recognize an invalid template method.
                    if let Ok(file) = parse_single_class_member_file(&member) {
                        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) =
                            &file.elements[0]
                        else {
                            panic!("expected a class");
                        };
                        assert!(def.members.methods.is_empty(), "{member}");
                    }
                }
            }
        }
    }

    #[test]
    fn test_parse_template_parameters_across_declaration_contexts() {
        let type_parameter = |name: &str, is_pack| TemplateParameter::Type {
            name: name.to_string(),
            is_pack,
        };
        let non_type_parameter =
            |value_type: &str, name: &str, is_pack| TemplateParameter::NonType {
                name: name.to_string(),
                value_type: value_type.to_string(),
                is_pack,
            };
        let cases = [
            // Type parameters.
            ("typename", type_parameter("", false)),
            ("class", type_parameter("", false)),
            ("typename...", type_parameter("", true)),
            ("class...", type_parameter("", true)),
            ("class... Ts", type_parameter("Ts", true)),
            ("typename... Ts", type_parameter("Ts", true)),
            ("Widget", type_parameter("Widget", false)),
            ("Ts...", type_parameter("Ts", true)),
            ("integer...", type_parameter("integer", true)),
            ("typename2", type_parameter("typename2", false)),
            ("boolName", type_parameter("boolName", false)),
            ("wchar_type...", type_parameter("wchar_type", true)),
            ("char16_type", type_parameter("char16_type", false)),
            ("class\tT", type_parameter("T", false)),
            ("typename\t...\tTs", type_parameter("Ts", true)),
            // Non-type parameters.
            ("bool", non_type_parameter("bool", "", false)),
            ("bool...", non_type_parameter("bool", "", true)),
            ("bool Enabled", non_type_parameter("bool", "Enabled", false)),
            ("bool... Flags", non_type_parameter("bool", "Flags", true)),
            ("wchar_t", non_type_parameter("wchar_t", "", false)),
            ("wchar_t...", non_type_parameter("wchar_t", "", true)),
            ("char8_t", non_type_parameter("char8_t", "", false)),
            ("char8_t...", non_type_parameter("char8_t", "", true)),
            ("char16_t", non_type_parameter("char16_t", "", false)),
            ("char16_t...", non_type_parameter("char16_t", "", true)),
            ("char32_t", non_type_parameter("char32_t", "", false)),
            ("char32_t...", non_type_parameter("char32_t", "", true)),
            ("int", non_type_parameter("int", "", false)),
            ("int...", non_type_parameter("int", "", true)),
            (
                "unsigned long...",
                non_type_parameter("unsigned long", "", true),
            ),
            ("int*", non_type_parameter("int*", "", false)),
            ("int&", non_type_parameter("int&", "", false)),
            ("int*...", non_type_parameter("int*", "", true)),
            ("std::size_t", non_type_parameter("std::size_t", "", false)),
            ("Widget*", non_type_parameter("Widget*", "", false)),
            ("Box<int>** P", non_type_parameter("Box<int>**", "P", false)),
            (
                "unsigned integer",
                non_type_parameter("unsigned", "integer", false),
            ),
            (
                "short integer",
                non_type_parameter("short", "integer", false),
            ),
            (
                "long longName",
                non_type_parameter("long", "longName", false),
            ),
            (
                "unsigned long integer",
                non_type_parameter("unsigned long", "integer", false),
            ),
            ("integer... Ns", non_type_parameter("integer", "Ns", true)),
            ("int* P", non_type_parameter("int*", "P", false)),
            ("int& R", non_type_parameter("int&", "R", false)),
            ("int** P", non_type_parameter("int**", "P", false)),
            ("int*& R", non_type_parameter("int*&", "R", false)),
            ("int * P", non_type_parameter("int *", "P", false)),
            ("int & R", non_type_parameter("int &", "R", false)),
            ("int*... Ps", non_type_parameter("int*", "Ps", true)),
            ("int&... Rs", non_type_parameter("int&", "Rs", true)),
            ("const int* P", non_type_parameter("const int*", "P", false)),
            ("const int N", non_type_parameter("const int", "N", false)),
            (
                "const Widget",
                non_type_parameter("const Widget", "", false),
            ),
            (
                "const unsigned long N",
                non_type_parameter("const unsigned long", "N", false),
            ),
            (
                "volatile int N",
                non_type_parameter("volatile int", "N", false),
            ),
            ("int const N", non_type_parameter("int const", "N", false)),
            ("int* const P", non_type_parameter("int* const", "P", false)),
            (
                "const volatile Widget* const* volatile P",
                non_type_parameter("const volatile Widget* const* volatile", "P", false),
            ),
            (
                "unsigned\tlong\tN",
                non_type_parameter("unsigned\tlong", "N", false),
            ),
            // Template-template parameters.
            (
                "template<bool, wchar_t, char8_t, char16_t, char32_t...> class C",
                TemplateParameter::Template {
                    name: "C".to_string(),
                    parameters: vec![
                        non_type_parameter("bool", "", false),
                        non_type_parameter("wchar_t", "", false),
                        non_type_parameter("char8_t", "", false),
                        non_type_parameter("char16_t", "", false),
                        non_type_parameter("char32_t", "", true),
                    ],
                    is_pack: false,
                },
            ),
            (
                "template<typename> class",
                TemplateParameter::Template {
                    name: String::new(),
                    parameters: vec![type_parameter("", false)],
                    is_pack: false,
                },
            ),
            (
                "template<typename> class...",
                TemplateParameter::Template {
                    name: String::new(),
                    parameters: vec![type_parameter("", false)],
                    is_pack: true,
                },
            ),
            (
                "template<typename> typename... Cs",
                TemplateParameter::Template {
                    name: "Cs".to_string(),
                    parameters: vec![type_parameter("", false)],
                    is_pack: true,
                },
            ),
            (
                "template<template<typename> class> class",
                TemplateParameter::Template {
                    name: String::new(),
                    parameters: vec![TemplateParameter::Template {
                        name: String::new(),
                        parameters: vec![type_parameter("", false)],
                        is_pack: false,
                    }],
                    is_pack: false,
                },
            ),
            (
                "template<int* P, int& R> class C",
                TemplateParameter::Template {
                    name: "C".to_string(),
                    parameters: vec![
                        non_type_parameter("int*", "P", false),
                        non_type_parameter("int&", "R", false),
                    ],
                    is_pack: false,
                },
            ),
            (
                "template<integer N> class C",
                TemplateParameter::Template {
                    name: "C".to_string(),
                    parameters: vec![non_type_parameter("integer", "N", false)],
                    is_pack: false,
                },
            ),
            (
                "template<className> class C",
                TemplateParameter::Template {
                    name: "C".to_string(),
                    parameters: vec![type_parameter("className", false)],
                    is_pack: false,
                },
            ),
            (
                "template<typename2...> typename... Cs",
                TemplateParameter::Template {
                    name: "Cs".to_string(),
                    parameters: vec![type_parameter("typename2", true)],
                    is_pack: true,
                },
            ),
        ];

        let cases = cases
            .into_iter()
            .map(|(parameters, expected)| (parameters, vec![expected]))
            .chain(std::iter::once((
                "typename T, int N, template<typename U> class C, typename... Ts, int... Ns, template<class V> class... Cs",
                vec![
                    type_parameter("T", false),
                    non_type_parameter("int", "N", false),
                    TemplateParameter::Template {
                        name: "C".to_string(),
                        parameters: vec![type_parameter("U", false)],
                        is_pack: false,
                    },
                    type_parameter("Ts", true),
                    non_type_parameter("int", "Ns", true),
                    TemplateParameter::Template {
                        name: "Cs".to_string(),
                        parameters: vec![type_parameter("V", false)],
                        is_pack: true,
                    },
                ],
            )));

        for (parameters, expected) in cases {
            let has_ambiguous_non_type = expected.iter().any(|parameter| {
                matches!(
                    parameter,
                    TemplateParameter::NonType { name, is_pack: false, .. } if name.is_empty()
                )
            });
            for (declaration, inferred_from_name) in [
                (format!("class Box<{parameters}>"), false),
                (format!("class \"Box<{parameters}>\""), true),
                (format!("class \"Box<{parameters}>\" as Alias"), true),
                (format!("class Alias as \"Box<{parameters}>\""), true),
                (
                    format!("class \"<<template>>\\n<{parameters}>\" as Box"),
                    false,
                ),
                (
                    format!("class Box as \"<<template>>\\n<{parameters}>\""),
                    false,
                ),
            ] {
                let file = parse_class_diagram_body(&declaration).expect(&declaration);
                let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &file.elements[0]
                else {
                    panic!("expected a class");
                };
                let expected_parameters = if inferred_from_name && has_ambiguous_non_type {
                    None
                } else {
                    Some(expected.as_slice())
                };
                assert_eq!(
                    def.template_parameters.as_deref(),
                    expected_parameters,
                    "{declaration}"
                );
            }

            let member = format!("+Run<{parameters}>()");
            let file = parse_single_class_member_file(&member).expect(&member);
            let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &file.elements[0]
            else {
                panic!("expected a class");
            };
            assert_eq!(def.members.methods.len(), 1, "{member}");
            assert_eq!(
                def.members.methods[0].template_parameters.as_deref(),
                Some(expected.as_slice()),
                "{member}"
            );
        }
    }

    #[test]
    fn test_parse_const_qualifier_after_empty_template_args() {
        let result = parse_single_class_member_file("+Get(): Foo<> <<const>>").unwrap();

        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class_def)) = &result.elements[0]
        else {
            panic!("expected class element");
        };

        assert_eq!(
            class_def.members.methods[0]
                .r#type
                .as_ref()
                .map(|ty| ty.raw_text.as_str()),
            Some("Foo<>")
        );
        assert_eq!(class_def.members.methods[0].modifiers, vec!["<<const>>"]);
    }

    #[test]
    fn test_parse_malformed_template_args_rejected() {
        assert!(parse_single_class_member_file("+f(Foo<int, > a)").is_err());
        assert!(parse_single_class_member_file("+f(Foo<<int>> a)").is_err());
    }

    #[test]
    fn test_parse_type_alias_source_location() {
        let input = r#"@startuml
            class A {
                +using Byte = std::uint8_t
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

        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(class_def)) = &result.elements[0]
        else {
            panic!("expected class element");
        };

        assert_eq!(class_def.members.type_aliases.len(), 1);
        assert_eq!(class_def.members.type_aliases[0].alias, "Byte");
        assert_eq!(
            class_def.members.type_aliases[0].original_type.raw_text,
            "std::uint8_t"
        );
        assert_eq!(class_def.members.type_aliases[0].source_location.line, 3);
    }

    #[test]
    fn test_parse_relationship_minimal() {
        let pair = PlantUmlCommonParser::parse(Rule::relationship, "A --> B")
            .unwrap()
            .next()
            .unwrap();

        let rel = super::parse_relationship(pair, SourceLocation::new("test.puml", 1));

        assert_eq!(rel.left, "A");
        assert_eq!(rel.right, "B");
        assert_eq!(rel.left_multiplicity, None);
        assert_eq!(rel.right_multiplicity, None);
        assert_eq!(rel.source_location.line, 1);
    }

    #[test]
    fn test_parse_relationship_colon_qualified_endpoint() {
        let pair = PlantUmlCommonParser::parse(Rule::relationship, "A --> ns::B")
            .unwrap()
            .next()
            .unwrap();

        let rel = super::parse_relationship(pair, SourceLocation::new("test.puml", 1));

        assert_eq!(rel.left, "A");
        assert_eq!(rel.right, "ns::B");
    }

    #[test]
    fn test_parse_relationship_colon_qualified_endpoint_with_multiplicities() {
        let pair = PlantUmlCommonParser::parse(Rule::relationship, "A \"1\" --> \"*\" ns::B")
            .unwrap()
            .next()
            .unwrap();

        let rel = super::parse_relationship(pair, SourceLocation::new("test.puml", 1));

        assert_eq!(rel.left, "A");
        assert_eq!(rel.right, "ns::B");
        assert_eq!(rel.left_multiplicity.as_deref(), Some("1"));
        assert_eq!(rel.right_multiplicity.as_deref(), Some("*"));
    }

    #[test]
    fn test_parse_relationship_label_is_not_mistaken_for_endpoint() {
        let pair = PlantUmlCommonParser::parse(Rule::relationship, "A --> B : ns::x")
            .unwrap()
            .next()
            .unwrap();

        let rel = super::parse_relationship(pair, SourceLocation::new("test.puml", 1));

        assert_eq!(rel.right, "B");
        assert_eq!(rel.label.as_deref(), Some("ns::x"));
    }

    #[test]
    fn test_parse_relationship_root_marker_endpoint() {
        let pair = PlantUmlCommonParser::parse(Rule::relationship, "A --> .B")
            .unwrap()
            .next()
            .unwrap();

        let rel = super::parse_relationship(pair, SourceLocation::new("test.puml", 1));

        assert_eq!(rel.right, ".B");
    }

    #[test]
    fn test_parse_relationship_double_colon_root_marker_endpoint() {
        let pair = PlantUmlCommonParser::parse(Rule::relationship, "::a::A --> ::b::B")
            .unwrap()
            .next()
            .unwrap();

        let rel = super::parse_relationship(pair, SourceLocation::new("test.puml", 1));

        assert_eq!(rel.left, "::a::A");
        assert_eq!(rel.right, "::b::B");
    }

    /// Both separators and both root markers, for every qualified position.
    const NAME_SPELLINGS: [&str; 6] = ["a.b.C", "a::b::C", "a.b::C", ".a.C", "::a::C", "::a.C"];

    #[test]
    fn test_declaration_names_accept_both_separators_and_root_markers() {
        for spelling in NAME_SPELLINGS {
            for keyword in ["class", "struct", "interface", "abstract class"] {
                let file = parse_class_diagram_body(&format!("{keyword} {spelling}"))
                    .unwrap_or_else(|error| panic!("`{keyword} {spelling}`: {error:?}"));
                let ClassUmlTopLevel::Type(element) = &file.elements[0] else {
                    panic!("`{keyword} {spelling}`: expected a type element");
                };
                let name = match element {
                    TypeDefinition::ClassLikeDef(def) => &def.declaration.name,
                    TypeDefinition::EntityDef(def) => &def.declaration.name,
                    TypeDefinition::EnumDef(def) => &def.declaration.name,
                };
                assert_eq!(name.internal, spelling, "`{keyword} {spelling}`");
            }

            let file = parse_class_diagram_body(&format!("enum {spelling}"))
                .unwrap_or_else(|error| panic!("`enum {spelling}`: {error:?}"));
            let ClassUmlTopLevel::Type(TypeDefinition::EnumDef(def)) = &file.elements[0] else {
                panic!("`enum {spelling}`: expected an enum");
            };
            assert_eq!(def.declaration.name.internal, spelling);
        }
    }

    #[test]
    fn test_package_and_namespace_names_accept_both_separators_and_root_markers() {
        for spelling in NAME_SPELLINGS {
            let file = parse_class_diagram_body(&format!("package {spelling} {{\n}}"))
                .unwrap_or_else(|error| panic!("`package {spelling}`: {error:?}"));
            let ClassUmlTopLevel::Package(package) = &file.elements[0] else {
                panic!("`package {spelling}`: expected a package");
            };
            assert_eq!(package.name.internal, spelling);

            let file = parse_class_diagram_body(&format!("namespace {spelling} {{\n}}"))
                .unwrap_or_else(|error| panic!("`namespace {spelling}`: {error:?}"));
            let ClassUmlTopLevel::Namespace(namespace) = &file.elements[0] else {
                panic!("`namespace {spelling}`: expected a namespace");
            };
            assert_eq!(namespace.name.internal, spelling);
        }
    }

    #[test]
    fn test_extends_and_implements_targets_accept_both_separators_and_root_markers() {
        for spelling in NAME_SPELLINGS {
            let file = parse_class_diagram_body(&format!(
                "class X extends {spelling}, Y implements {spelling}, Z"
            ))
            .unwrap_or_else(|error| panic!("{spelling}: {error:?}"));
            let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &file.elements[0]
            else {
                panic!("{spelling}: expected a class");
            };
            let ClassLikeKind::Class {
                extends,
                implements,
                ..
            } = &def.kind
            else {
                panic!("{spelling}: expected a class kind");
            };
            assert_eq!(extends, &[spelling, "Y"], "{spelling}");
            assert_eq!(implements, &[spelling, "Z"], "{spelling}");
        }
    }

    #[test]
    fn test_abstract_class_accepts_multiple_spaces() {
        let file = parse_class_diagram_body("abstract    class AbstractType").unwrap();
        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &file.elements[0] else {
            panic!("expected an abstract class");
        };
        let ClassLikeKind::Class { is_abstract, .. } = def.kind else {
            panic!("expected a class kind");
        };

        assert!(is_abstract);
    }

    #[test]
    fn test_relationship_endpoints_accept_both_separators_and_root_markers() {
        for spelling in NAME_SPELLINGS {
            let file = parse_class_diagram_body(&format!("{spelling} --> {spelling}"))
                .unwrap_or_else(|error| panic!("{spelling}: {error:?}"));
            assert_eq!(file.relationships.len(), 1, "{spelling}");
            assert_eq!(file.relationships[0].left, spelling);
            assert_eq!(file.relationships[0].right, spelling);
        }
    }

    #[test]
    fn test_malformed_qualified_names_are_rejected() {
        for spelling in ["a..b", "a::", "a:::b", "::", ".", "a.", "a.::b"] {
            for body in [
                format!("class {spelling}"),
                format!("package {spelling} {{\n}}"),
                format!("class X extends {spelling}"),
            ] {
                let result = parse_class_diagram_body(&body);
                assert!(result.is_err(), "`{body}` must not parse: {result:?}");
            }
        }
    }

    #[test]
    fn test_quoted_alias_is_the_name() {
        let file = parse_class_diagram_body("class F as \"a::Foo\"\npackage p as \"b::q\" {\n}")
            .expect("must parse");

        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &file.elements[0] else {
            panic!("expected a class");
        };
        assert_eq!(def.declaration.name.internal, "a::Foo");
        assert_eq!(def.declaration.name.alias.as_deref(), Some("F"));

        let ClassUmlTopLevel::Package(package) = &file.elements[1] else {
            panic!("expected a package");
        };
        assert_eq!(package.name.internal, "b::q");
        assert_eq!(package.name.alias.as_deref(), Some("p"));
    }

    #[test]
    fn test_quoted_name_with_bare_alias_keeps_roles() {
        let file = parse_class_diagram_body("class \"a::Foo\" as F").expect("must parse");

        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &file.elements[0] else {
            panic!("expected a class");
        };
        assert_eq!(def.declaration.name.internal, "a::Foo");
        assert_eq!(def.declaration.name.alias.as_deref(), Some("F"));
    }

    #[test]
    fn test_quoted_alias_is_the_name_even_when_the_name_is_quoted() {
        let file = parse_class_diagram_body("class \"A\" as \"b::B\"").expect("must parse");

        let ClassUmlTopLevel::Type(TypeDefinition::ClassLikeDef(def)) = &file.elements[0] else {
            panic!("expected a class");
        };
        assert_eq!(def.declaration.name.internal, "b::B");
        assert_eq!(def.declaration.name.alias.as_deref(), Some("A"));
    }

    #[test]
    fn test_single_line_note_alias_registers_ignored_note() {
        let input = "note \"Synchronised access only\" as SyncNote";
        let normalized = normalize_multiline_member_signatures(input);
        let pair = PlantUmlCommonParser::parse(Rule::note_declaration, input)
            .unwrap()
            .next()
            .unwrap();

        let mut session = make_test_session(&normalized);
        session.parse_top_level_element(pair).unwrap();

        assert!(session.ignored_notes.contains("SyncNote"));
    }

    #[test]
    fn test_multiline_note_alias_registers_ignored_note() {
        let input = "note as SyncNote\n  Synchronised access only\nend note\n";
        let normalized = normalize_multiline_member_signatures(input);
        let pair = PlantUmlCommonParser::parse(Rule::note_declaration, input)
            .unwrap()
            .next()
            .unwrap();

        let mut session = make_test_session(&normalized);
        session.parse_top_level_element(pair).unwrap();

        assert!(session.ignored_notes.contains("SyncNote"));
    }

    #[test]
    fn test_enum_value_all_cases() {
        // literal
        let pair = PlantUmlCommonParser::parse(Rule::enum_value, "= 1")
            .unwrap()
            .next()
            .unwrap();
        match super::ClassParseSession::parse_enum_value(pair) {
            EnumValue::Literal(v) => assert_eq!(v, "1"),
            _ => panic!(),
        }

        // description
        let pair = PlantUmlCommonParser::parse(Rule::enum_value, ": ok")
            .unwrap()
            .next()
            .unwrap();
        match super::ClassParseSession::parse_enum_value(pair) {
            EnumValue::Description(v) => assert_eq!(v, "ok"),
            _ => panic!(),
        }
    }
}
