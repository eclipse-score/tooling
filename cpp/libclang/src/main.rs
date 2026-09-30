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

use clang::diagnostic::Severity;
use clap::Parser as ClapParser;
use env_logger::Builder;
use log::{debug, error, warn, LevelFilter};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use class_diagram::{ClassDiagram, FreeFunctionDecl, SimpleEntity};
use class_serializer::ClassSerializer;

use utils::{render_entity_tree, write_debug_json, write_entity_tree, write_fbs_output};
use visit_tu::{
    is_external_dependency_path, CallableDeclarationKey, EntityMapExt, FunctionDef,
    SourceEntityKey, SourceFileCache, VisitContext, Visitor,
};

#[derive(ClapParser, Debug)]
#[command(name = "cpp_parser")]
#[command(author = "Eclipse Foundation Contributors")]
#[command(version = "0.1.0")]
#[command(about = "Parse C/C++ source files using libclang and extract AST info")]
struct Args {
    /// Input C/C++ source files
    #[arg(long, required = true, num_args = 1..)]
    input: Vec<PathBuf>,

    /// Class diagram FlatBuffer output path (internal use only)
    #[arg(long, hide = true)]
    class_fbs_output: PathBuf,

    /// Additional compiler arguments (e.g., -I/path/to/includes)
    #[arg(short = 'X', long = "extra-arg", allow_hyphen_values = true)]
    extra_args: Vec<String>,

    /// Debug JSON output path (internal use only)
    #[arg(long, hide = true)]
    debug_json_output: Option<PathBuf>,

    /// Do not fail the action when a translation unit has parse errors
    /// (fatal libclang diagnostics or an outright parse failure). The
    /// resulting AST may then contain clang's own error-recovery
    /// placeholders (e.g. an unresolved field type reported as `int`).
    #[arg(long)]
    allow_parse_errors: bool,
}

#[derive(Default)]
struct ParseOutputs {
    types: BTreeMap<String, SimpleEntity>,
    free_function_declarations: Vec<FreeFunctionDecl>,
    functions: Vec<FunctionDef>,
}

#[derive(Default)]
struct ParseState {
    source_files: SourceFileCache,
    seen_free_function_declarations: HashSet<CallableDeclarationKey>,
    seen_method_declarations: HashSet<CallableDeclarationKey>,
    seen_function_definitions: HashSet<SourceEntityKey>,
}

impl ParseOutputs {
    fn extend_from_ctx(&mut self, ctx: VisitContext) {
        debug!(
            "Visited TU, extracted {} types, {} functions",
            ctx.types.len(),
            ctx.functions.len()
        );

        for (type_name, entity) in ctx.types {
            debug!("Type {}:\n{:#?}", type_name, entity);
            self.types.insert_or_merge_type(type_name, entity);
        }
        self.free_function_declarations
            .extend(
                ctx.free_function_declarations
                    .into_iter()
                    .map(|declaration| {
                        debug!(
                            "Free function declaration: {}",
                            declaration.declaration.qualified_name()
                        );
                        declaration.declaration
                    }),
            );
        self.functions.extend(
            ctx.functions
                .into_iter()
                .map(|function| function.definition),
        );
    }
}

fn init_logging() {
    let log_level = std::env::var("LIBCLANG_LOG")
        .ok()
        .and_then(|value| value.parse::<LevelFilter>().ok())
        .unwrap_or(LevelFilter::Error);

    Builder::new().filter_level(log_level).init();
}

fn init_libclang() -> clang::Clang {
    debug!("=== libclang Information ===");
    debug!("Command line: {:?}", std::env::args().collect::<Vec<_>>());

    if let Ok(path) = std::env::var("LIBCLANG_PATH") {
        debug!("LIBCLANG_PATH: {}", path);
    }

    let clang = match clang::Clang::new() {
        Ok(c) => {
            debug!("Successfully loaded libclang");
            c
        }
        Err(e) => {
            error!("Failed to load libclang: {}", e);
            std::process::exit(1);
        }
    };

    debug!("libclang version: {}", clang::get_version());
    debug!("Using Bazel's LLVM toolchain with clang-rs wrapper");
    clang
}

fn init_clang_index(clang: &clang::Clang) -> clang::Index<'_> {
    let index = clang::Index::new(clang, false, true);
    debug!("Created clang index");
    index
}

fn parse_file(
    file: &Path,
    compilation_flags: &[String],
    index: &clang::Index,
    trace_output_dir: Option<&Path>,
    allow_parse_errors: bool,
    state: &mut ParseState,
    outputs: &mut ParseOutputs,
) -> bool {
    debug!("Parsing TU: {:?}", file);

    if let Some(path_str) = file.to_str() {
        if is_external_dependency_path(path_str) {
            debug!("Skipping external dependency file: {:?}", file);
            return true;
        }
    };

    let parse_result = index.parser(file).arguments(compilation_flags).parse();

    match parse_result {
        Ok(parsed) => {
            let diagnostics = parsed.get_diagnostics();
            let mut has_errors = false;
            if !diagnostics.is_empty() {
                debug!("Diagnostics: {}", diagnostics.len());
                for diagnostic in &diagnostics {
                    debug!("Diagnostic: {:?}", diagnostic);
                    if matches!(diagnostic.get_severity(), Severity::Error | Severity::Fatal) {
                        has_errors = true;
                        // A tolerated parse error still needs surfacing, but not at
                        // `error!` level: the caller opted in to continuing anyway
                        // (--allow-parse-errors), so this is expected, not fatal.
                        if allow_parse_errors {
                            warn!("{}", diagnostic);
                        } else {
                            error!("{}", diagnostic);
                        }
                    }
                }
            }

            let entity = parsed.get_entity();
            debug!("Parsed {:?} successfully", parsed);
            if log::log_enabled!(log::Level::Trace) {
                if let Some(trace_output_dir) = trace_output_dir {
                    let ast_file_output_path = trace_output_dir.join("libclang_parsed_ast.txt");
                    let entity_tree = render_entity_tree(&entity, 0);
                    write_entity_tree(&ast_file_output_path, &entity_tree);
                }
            }

            let mut ctx = VisitContext::default();
            let mut visitor = Visitor::new(
                &mut ctx,
                &mut state.source_files,
                &mut state.seen_free_function_declarations,
                &mut state.seen_method_declarations,
                &mut state.seen_function_definitions,
            );
            visitor.visit(entity);
            outputs.extend_from_ctx(ctx);
            !has_errors
        }
        Err(e) => {
            error!("Failed to parse {:?}: {:?}", file, e);
            false
        }
    }
}

fn serialize_class_diagram(
    output_path: &Path,
    entities: BTreeMap<String, SimpleEntity>,
    free_functions: Vec<FreeFunctionDecl>,
) -> Result<(), std::io::Error> {
    let entities: Vec<_> = entities.into_values().collect();
    let class_diagram = ClassDiagram {
        name: String::new(), // no name for c++ side
        entities,
        free_functions,
    };

    let output_fbs = ClassSerializer::serialize(&class_diagram);
    write_fbs_output(output_path, &output_fbs)?;

    Ok(())
}

fn ensure_output_parent_exists(path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    let clang = init_libclang();
    let index = init_clang_index(&clang);

    let command_line_args = Args::parse();
    let mut outputs = ParseOutputs::default();
    let mut state = ParseState::default();

    ensure_output_parent_exists(&command_line_args.class_fbs_output)?;
    if let Some(debug_json_output) = &command_line_args.debug_json_output {
        ensure_output_parent_exists(debug_json_output)?;
    }

    let trace_output_dir = command_line_args.class_fbs_output.parent().or_else(|| {
        command_line_args
            .debug_json_output
            .as_deref()
            .and_then(Path::parent)
    });

    let mut all_parsed_cleanly = true;
    for file in &command_line_args.input {
        let compilation_flags = &command_line_args.extra_args;

        let parsed_cleanly = parse_file(
            file,
            compilation_flags,
            &index,
            trace_output_dir,
            command_line_args.allow_parse_errors,
            &mut state,
            &mut outputs,
        );
        all_parsed_cleanly &= parsed_cleanly;
    }

    if let Some(debug_json_output) = &command_line_args.debug_json_output {
        write_debug_json(
            debug_json_output,
            &outputs.types,
            (!outputs.free_function_declarations.is_empty())
                .then_some(&outputs.free_function_declarations),
            &outputs.functions,
        )?;
    }

    serialize_class_diagram(
        &command_line_args.class_fbs_output,
        outputs.types,
        outputs.free_function_declarations,
    )?;

    if !all_parsed_cleanly && !command_line_args.allow_parse_errors {
        return Err(
            "one or more translation units had parse errors (pass --allow-parse-errors to continue anyway)"
                .into(),
        );
    }

    Ok(())
}
