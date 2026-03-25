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

use clap::{ArgGroup, Parser, ValueEnum};
use env_logger::Builder;
use log::{debug, error, warn};
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use puml_lobster::write_lobster_to_file;
use puml_parser::{
    DiagramParser, Preprocessor, PumlClassParser, PumlComponentParser, PumlSequenceParser,
};
use puml_resolver::{ComponentResolver, DiagramResolver};
use puml_serializer::ComponentSerializer;
use puml_utils::{write_fbs_to_file, write_json_to_file, write_placeholder_file, LogLevel};

/// CLI wrapper for LogLevel that implements ValueEnum
#[derive(Copy, Clone, ValueEnum, Debug)]
enum CliLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<CliLogLevel> for LogLevel {
    fn from(cli_level: CliLogLevel) -> Self {
        match cli_level {
            CliLogLevel::Error => LogLevel::Error,
            CliLogLevel::Warn => LogLevel::Warn,
            CliLogLevel::Info => LogLevel::Info,
            CliLogLevel::Debug => LogLevel::Debug,
            CliLogLevel::Trace => LogLevel::Trace,
        }
    }
}

/// PlantUML parser CLI tool
#[derive(Parser, Debug)]
#[command(name = "puml_parser_cli")]
#[command(version = "1.0")]
#[command(about = "Parse and analyze PlantUML component diagrams", long_about = None)]
#[command(group(
    ArgGroup::new("input")
        .required(true)
        .multiple(true)
        .args(&["file", "folders"]),
))]
struct Args {
    /// One or more PUML files to parse (can be repeated)
    #[arg(long)]
    file: Vec<String>,

    /// Folder containing PUML files
    #[arg(long)]
    folders: Option<String>,

    /// Log level: error, warn, info, debug, trace
    #[arg(long, value_enum, default_value = "warn")]
    log_level: CliLogLevel,

    /// Specify Grammar / Diagram type explicitly
    #[arg(long, value_enum, default_value = "none")]
    diagram_type: DiagramType,

    /// Output directory for generated FlatBuffers binary files.
    /// When omitted, no FlatBuffers files are written.
    #[arg(long)]
    fbs_output_dir: Option<String>,

    /// Output directory for generated lobster files (optional).
    /// When set, a <stem>.lobster is written for each diagram that resolves
    /// to a Component model (independent of --fbs-output-dir). On resolve
    /// errors a placeholder empty .lobster is written so the build output
    /// set is always complete.
    #[arg(long)]
    lobster_output_dir: Option<String>,
}

#[derive(Copy, Clone, ValueEnum, Debug)]
enum DiagramType {
    None,
    Component,
    Class,
    Sequence,
}

#[allow(dead_code)] // Class and Sequence variants are WIP
#[derive(Debug, Serialize)]
enum ParsedDiagram {
    Component(puml_parser::CompPumlDocument),
    Class(puml_parser::ClassUmlFile),
    Sequence(puml_parser::SeqPumlDocument),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let log_level: LogLevel = args.log_level.into();
    Builder::new()
        .filter_level(log_level.to_level_filter())
        .init();
    let emit_debug_json = log_level.to_level_filter() >= log::LevelFilter::Debug;

    let fbs_output_dir: Option<PathBuf> = if let Some(dir) = &args.fbs_output_dir {
        let p = PathBuf::from(dir);
        fs::create_dir_all(&p)?;
        Some(p)
    } else {
        None
    };

    let lobster_output_dir: Option<PathBuf> = match &args.lobster_output_dir {
        Some(dir) => {
            let p = PathBuf::from(dir);
            fs::create_dir_all(&p)?;
            Some(p)
        }
        None => None,
    };

    let file_list = collect_files_from_args(&args)?;

    if file_list.is_empty() {
        return Err("No valid PUML files found.".into());
    }
    debug!("Collected {} puml files.", file_list.len());

    debug!("Preprocessing: include expansion");
    let mut preprocessor = Preprocessor::new();
    let preprocessed_files = preprocessor.preprocess(&file_list, log_level)?;

    debug!("Parsing started");
    for (path, content) in &preprocessed_files {
        let parsed_content =
            parse_puml_file(path, content, log_level, args.diagram_type).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Parse error in {}: {}", path.display(), e),
                )
            })?;
        if emit_debug_json {
            if let Some(ref dir) = fbs_output_dir {
                write_json_to_file(&parsed_content, path, dir, "raw.ast")?;
            }
        }

        match resolve_parsed_diagram(parsed_content) {
            Ok(logic_result) => {
                debug!(
                    "Successfully resolved PlantUML document: {}",
                    path.display()
                );
                if emit_debug_json {
                    if let Some(ref dir) = fbs_output_dir {
                        write_json_to_file(&logic_result, path, dir, "logic.ast")?;
                    }
                }

                let source_file = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                let fbs_buffer = serialize_resolved_diagram(&logic_result, source_file);
                if let Some(ref dir) = fbs_output_dir {
                    write_fbs_to_file(&fbs_buffer, path, dir)?;
                }

                if let (ResolvedDiagram::Component(ref model), Some(ldir)) =
                    (&logic_result, &lobster_output_dir)
                {
                    write_lobster_to_file(model, path, ldir)?;
                }
            }
            Err(e) => {
                error!("Resolve error in {}: {}", path.display(), e);
                warn!(
                    "Skipping file due to unimplemented diagram type: {}",
                    path.display()
                );
                // Create empty placeholder files so the build continues
                if let Some(ref dir) = fbs_output_dir {
                    write_placeholder_file(path, dir)?;
                }
                if let Some(ref ldir) = lobster_output_dir {
                    write_lobster_to_file(&HashMap::new(), path, ldir)?;
                }
            }
        }
    }

    debug!("Parsing completed");
    Ok(())
}

fn serialize_resolved_diagram(resolved_content: &ResolvedDiagram, source_file: &str) -> Vec<u8> {
    match resolved_content {
        ResolvedDiagram::Component(resolved_content) => {
            ComponentSerializer::serialize(resolved_content, source_file)
        } // ResolvedDiagram::Class(_) => {     // placeholder
          //     /* class serializer */
          // }
          // ResolvedDiagram::Sequence(_) => {    // placeholder
          //     /* sequence serializer */
          // }
    }
}

#[derive(Debug, Serialize)]
pub enum ResolvedDiagram {
    Component(HashMap<String, puml_resolver::LogicComponent>),
    // Class(ClassLogic),       // placeholder
    // Sequence(SequenceLogic), // placeholder
}

fn resolve_parsed_diagram(
    parsed_content: ParsedDiagram,
) -> Result<ResolvedDiagram, Box<dyn std::error::Error>> {
    match parsed_content {
        ParsedDiagram::Component(parsed_content) => {
            let mut resolver = ComponentResolver::new();
            puml_resolver(&mut resolver, &parsed_content).map(ResolvedDiagram::Component)
        }
        ParsedDiagram::Class(_) => {
            /* class resolver */
            Err("Class diagrams not implemented".into())
        }
        ParsedDiagram::Sequence(_) => {
            /* sequence resolver */
            Err("Sequence diagrams not implemented".into())
        }
    }
}

fn puml_resolver<Resolver>(
    resolver: &mut Resolver,
    parsed_content: &Resolver::Document,
) -> Result<Resolver::Output, Box<dyn std::error::Error>>
where
    Resolver: DiagramResolver,
    Resolver::Output: std::fmt::Debug,
    Resolver::Error: std::error::Error + 'static,
{
    let logic_result = resolver
        .visit_document(parsed_content)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    Ok(logic_result)
}

fn parse_with_parser<P>(
    parser: &mut P,
    path: &Rc<PathBuf>,
    content: &str,
    log_level: LogLevel,
) -> Result<P::Output, Box<dyn std::error::Error>>
where
    P: DiagramParser,
    P::Output: std::fmt::Debug,
    P::Error: std::error::Error + 'static,
{
    let parsed_content = parser
        .parse_file(path, content, log_level)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    debug!("Successfully parsed PlantUML document: {}", path.display());
    Ok(parsed_content)
}

// lobster-trace: Tools.ArchitectureModelingSyntax
fn parse_puml_file(
    path: &Rc<PathBuf>,
    content: &str,
    log_level: LogLevel,
    diagram_type: DiagramType,
) -> Result<ParsedDiagram, Box<dyn std::error::Error>> {
    match diagram_type {
        DiagramType::Component => {
            parse_with_parser(&mut PumlComponentParser, path, content, log_level)
                .map(ParsedDiagram::Component)
        }
        DiagramType::Class => parse_with_parser(&mut PumlClassParser, path, content, log_level)
            .map(ParsedDiagram::Class),
        DiagramType::Sequence => {
            parse_with_parser(&mut PumlSequenceParser, path, content, log_level)
                .map(ParsedDiagram::Sequence)
        }
        DiagramType::None => parse_in_order(path, content, log_level),
    }
}

type ParserFn =
    fn(&Rc<PathBuf>, &str, LogLevel) -> Result<ParsedDiagram, Box<dyn std::error::Error>>;

fn parse_in_order(
    path: &Rc<PathBuf>,
    content: &str,
    log_level: LogLevel,
) -> Result<ParsedDiagram, Box<dyn std::error::Error>> {
    let parsers: &[(&str, ParserFn)] = &[
        ("Component", |p, c, l| {
            parse_with_parser(&mut PumlComponentParser, p, c, l).map(ParsedDiagram::Component)
        }),
        ("Class", |p, c, l| {
            parse_with_parser(&mut PumlClassParser, p, c, l).map(ParsedDiagram::Class)
        }),
        ("Sequence", |p, c, l| {
            parse_with_parser(&mut PumlSequenceParser, p, c, l).map(ParsedDiagram::Sequence)
        }),
    ];

    for (parser_name, parser) in parsers {
        if let Ok(ast) = parser(path, content, log_level) {
            debug!("Successfully detected as {} diagram", parser_name);
            return Ok(ast);
        }
    }

    Err(format!(
        "Failed to parse {} with any available parser",
        path.display()
    )
    .into())
}

fn collect_files_from_args(
    args: &Args,
) -> Result<HashSet<Rc<PathBuf>>, Box<dyn std::error::Error>> {
    let mut file_list: HashSet<Rc<PathBuf>> = HashSet::new();

    // Collect individual files from --file arguments (may be repeated)
    for file_path in &args.file {
        add_single_file(Path::new(file_path), &mut file_list)?;
    }

    // Collect files from folders using --folders argument
    if let Some(folder_path) = &args.folders {
        collect_puml_files_from_folder(Path::new(folder_path), &mut file_list)?;
    }

    Ok(file_list)
}

fn resolve_path(path: &Path) -> PathBuf {
    // When running with 'bazel run', use BUILD_WORKSPACE_DIRECTORY
    let base_dir = std::env::var("BUILD_WORKSPACE_DIRECTORY")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn add_single_file(
    path: &Path,
    file_list: &mut HashSet<Rc<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let abs_path = resolve_path(path);

    if !abs_path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()).into());
    }
    if abs_path.extension().and_then(|ext| ext.to_str()) != Some("puml") {
        return Err(format!("File is not a .puml file: {}", path.display()).into());
    }
    file_list.insert(Rc::new(abs_path));
    Ok(())
}

fn collect_puml_files_from_folder(
    dir: &Path,
    file_list: &mut HashSet<Rc<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let abs_dir = resolve_path(dir);

    if !abs_dir.is_dir() {
        return Err(format!("Path is not a directory: {}", dir.display()).into());
    }
    collect_puml_files(&abs_dir, file_list)?;
    Ok(())
}

fn collect_puml_files(
    dir: &Path,
    file_list: &mut HashSet<Rc<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_puml_files(&path, file_list)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("puml") {
            file_list.insert(Rc::new(path.to_path_buf()));
        }
    }
    Ok(())
}
