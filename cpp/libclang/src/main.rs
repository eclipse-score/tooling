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
use clang::{Entity, EntityKind, EntityVisitResult};
use clap::Parser as ClapParser;
use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
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

    /// Print verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn parse_file(
    file: &PathBuf,
    compilation_flags: &[String],
    index: &clang::Index,
    ast_file_output_path: &PathBuf,
    all_classes: &mut BTreeMap<String, context::TypeMapValue>,
    all_functions: &mut Vec<FunctionDef>,
) {
    println!("Parsing TU: {:?}", file);

    if let Some(path_str) = file.to_str() {
        if visitor::is_external_dependency_path(path_str) {
            println!("  Skipping external dependency file: {:?}", file);
            return;
        }
    };

    let parse_result = index.parser(file).arguments(compilation_flags).parse();

    match parse_result {
        Ok(parsed) => {
            println!("  Parsed successfully, parsed is {:?}", parsed);

            let diagnostics = parsed.get_diagnostics();
            if !diagnostics.is_empty() {
                println!("  Diagnostics: {}", diagnostics.len());
            }

            let entity = parsed.get_entity();
            print_entity(&entity, 0, PrintMode::File(ast_file_output_path));
            print_entity(&entity, 0, PrintMode::Stdout);

            let mut ctx = VisitContext::default();
            let mut visitor = Visitor::new(&mut ctx);
            visitor.visit(entity);
            println!(
                "  Visited TU, extracted {} classes, {} functions",
                ctx.types.len(),
                ctx.functions.len()
            );
            for (class_name, logic_class) in &ctx.types {
                println!("  - class: {}", class_name);
                println!("{:#?}", logic_class);
                all_classes.insert(class_name.clone(), logic_class.clone());
            }
            all_functions.extend(ctx.functions);
        }
        Err(e) => {
            eprintln!("  Failed to parse {:?}: {:?}", file, e);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== libclang Information ===\n");
    println!("Command line: {:?}", std::env::args().collect::<Vec<_>>());

    if let Ok(path) = std::env::var("LIBCLANG_PATH") {
        println!("LIBCLANG_PATH: {}", path);
    }

    // Load clang - keep it alive for the entire scope
    let clang = match clang::Clang::new() {
        Ok(c) => {
            println!("✓ Successfully loaded libclang\n");
            c
        }
        Err(e) => {
            eprintln!("Failed to load libclang: {}", e);
            std::process::exit(1);
        }
    };

    // All operations use the clang-rs wrapper API
    let index = clang::Index::new(&clang, false, true);
    println!("✓ Created clang index");
    println!("libclang version: {}", clang::get_version());

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
    println!("Wrote AST JSON to {:?}", command_line_args.output);
    println!("\n Using Bazel's LLVM toolchain with clang-rs wrapper!");

    Ok(())
}

enum PrintMode<'a> {
    Stdout,
    File(&'a PathBuf),
}

fn print_entity(entity: &Entity, level: usize, print_mode: PrintMode) {
    match print_mode {
        PrintMode::Stdout => {
            let mut stdout = std::io::stdout().lock();
            print_entity_to(entity, level, &mut stdout)
        }
        PrintMode::File(path) => {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .unwrap_or_else(|e| {
                    panic!("Failed to open file {:?} for writing: {}", path, e);
                });
            let mut file_out = BufWriter::new(file);
            print_entity_to(entity, level, &mut file_out)
        }
    }
}

fn print_entity_to(entity: &Entity, level: usize, out: &mut dyn Write) {
    let indent = "  ".repeat(level);
    let kind = entity.get_kind();
    let spelling = match kind {
        EntityKind::AccessSpecifier => entity
            .get_accessibility()
            .map(|a| format!("{:?}", a).to_lowercase())
            .unwrap_or_default(),
        _ => entity.get_name().unwrap_or_default(),
    };
    let output = format!("{}{:?} | {}\n", indent, kind, spelling);

    // we soft fail since this is just a diagnostic print and should not crush the main programm
    out.write_all(output.as_bytes()).unwrap_or_else(|e| {
        eprintln!("Failed to write entity info: {}", e);
    });
    out.flush().unwrap_or_else(|e| {
        eprintln!("Failed to flush output: {}", e);
    });

    entity.visit_children(|child, _parent| {
        print_entity_to(&child, level + 1, out);
        EntityVisitResult::Continue
    });
}
