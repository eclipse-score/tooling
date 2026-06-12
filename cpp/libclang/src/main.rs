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
use std::path::PathBuf;

use utils::{render_entity_tree, write_entity_tree};
use visit_tu::context;
use visit_tu::visitor;
use visit_tu::{FunctionDef, VisitContext, Visitor};

#[derive(ClapParser, Debug)]
#[command(name = "cpp_parser")]
#[command(author = "Eclipse Foundation Contributors")]
#[command(version = "0.1.0")]
#[command(about = "Parse C/C++ source files using libclang and extract AST info")]
struct Args {
    /// Input C/C++ source files
    input: Vec<PathBuf>,

    /// Output file path
    #[arg(short, long)]
    output: PathBuf,

    /// Additional compiler arguments (e.g., -I/path/to/includes)
    #[arg(short = 'X', long = "extra-arg", allow_hyphen_values = true)]
    extra_args: Vec<String>,

    /// Output JSON format for debugging (internal use only)
    #[arg(long, hide = true)]
    json: bool,
}

fn parse_file(
    file: &PathBuf,
    compilation_flags: &[String],
    index: &clang::Index,
    ast_file_output_path: &PathBuf,
    all_classes: &mut BTreeMap<String, context::TypeMapValue>,
    all_functions: &mut Vec<FunctionDef>,
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
                let entity_tree = render_entity_tree(&entity, 0);
                write_entity_tree(ast_file_output_path, &entity_tree);
            }

            let mut ctx = VisitContext::default();
            let mut visitor = Visitor::new(&mut ctx);
            visitor.visit(entity);
            debug!(
                "Visited TU, extracted {} classes, {} functions",
                ctx.types.len(),
                ctx.functions.len()
            );
            for (class_name, logic_class) in &ctx.types {
                debug!("Class {}:\n{:#?}", class_name, logic_class);
                all_classes.insert(class_name.clone(), logic_class.clone());
            }
            all_functions.extend(ctx.functions);
        }
        Err(e) => {
            error!("Failed to parse {:?}: {:?}", file, e);
        }
    }
}

fn parse_log_level_from_env() -> LevelFilter {
    std::env::var("LIBCLANG_LOG")
        .ok()
        .and_then(|value| value.parse::<LevelFilter>().ok())
        .unwrap_or(LevelFilter::Error)
}

fn init_logging() {
    Builder::new()
        .filter_level(parse_log_level_from_env())
        .init();
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

fn init_clang_index(clang: &clang::Clang) -> clang::Index {
    let index = clang::Index::new(clang, false, true);
    debug!("Created clang index");
    index
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    let clang = init_libclang();
    let index = init_clang_index(&clang);

    let command_line_args = Args::parse();
    let mut all_classes = BTreeMap::new();
    let mut all_functions: Vec<FunctionDef> = Vec::new();

    let ast_file_output_path = command_line_args
        .output
        .parent()
        .ok_or("Output path must have a parent directory")?
        .join("libclang_parsed_ast.txt");

    for file in &command_line_args.input {
        let compilation_flags = &command_line_args.extra_args;

        parse_file(
            file,
            compilation_flags,
            &index,
            &ast_file_output_path,
            &mut all_classes,
            &mut all_functions,
        );
    }
    let mut output = serde_json::Map::new();
    output.insert("classes".to_owned(), serde_json::to_value(&all_classes)?);
    output.insert(
        "functions".to_owned(),
        serde_json::to_value(&all_functions)?,
    );
    let output_json = serde_json::to_string_pretty(&output)?;
    fs::write(&command_line_args.output, output_json)?;
    debug!("Wrote AST JSON to {:?}", command_line_args.output);

    Ok(())
}
