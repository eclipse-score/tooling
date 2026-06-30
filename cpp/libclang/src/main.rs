///////////////////////////////////////////////////////////////////////////////////
// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
// SPDX-License-Identifier: Apache-2.0
////////////////////////////////////////////////////////////////////////////////////
use clap::Parser as ClapParser;
use env_logger::Builder;
use log::{debug, error, LevelFilter};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use class_diagram::{ClassDiagram, SimpleEntity};
use class_serializer::ClassSerializer;

use utils::{render_entity_tree, write_debug_json, write_entity_tree, write_fbs_output};
use visit_tu::visitor;
use visit_tu::{FunctionDef, VisitContext, Visitor};

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
}

#[derive(Default)]
struct ParseOutputs {
    types: BTreeMap<String, SimpleEntity>,
    functions: Vec<FunctionDef>,
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
            self.types.insert(type_name, entity);
        }

        self.functions.extend(ctx.functions);
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
    outputs: &mut ParseOutputs,
) {
    debug!("Parsing TU: {:?}", file);

    if let Some(path_str) = file.to_str() {
        if visitor::is_external_dependency_path(path_str) {
            debug!("Skipping external dependency file: {:?}", file);
            return;
        }
    };

    let parse_result = index.parser(file).arguments(compilation_flags).parse();

    match parse_result {
        Ok(parsed) => {
            let diagnostics = parsed.get_diagnostics();
            if !diagnostics.is_empty() {
                debug!("Diagnostics: {}", diagnostics.len());
                for diagnostic in &diagnostics {
                    debug!("Diagnostic: {:?}", diagnostic);
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
            let mut visitor = Visitor::new(&mut ctx);
            visitor.visit(entity);
            outputs.extend_from_ctx(ctx);
        }
        Err(e) => {
            error!("Failed to parse {:?}: {:?}", file, e);
        }
    }
}

fn serialize_class_diagram(
    output_path: &Path,
    entities: BTreeMap<String, SimpleEntity>,
) -> Result<(), std::io::Error> {
    let entities: Vec<_> = entities.into_values().collect();
    let class_diagram = ClassDiagram {
        name: String::new(), // no name for c++ side
        entities,
        relationships: Vec::new(), // relationships are included at the entity level for c++
        source_files: Vec::new(),  // source files are not tracked at the diagram level for c++
        version: None,
    };

    let output_fbs = ClassSerializer::serialize(&class_diagram, "");
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

    for file in &command_line_args.input {
        let compilation_flags = &command_line_args.extra_args;

        parse_file(
            file,
            compilation_flags,
            &index,
            trace_output_dir,
            &mut outputs,
        );
    }

    if let Some(debug_json_output) = &command_line_args.debug_json_output {
        write_debug_json(debug_json_output, &outputs.types, &outputs.functions)?;
    }

    serialize_class_diagram(&command_line_args.class_fbs_output, outputs.types)?;

    Ok(())
}
