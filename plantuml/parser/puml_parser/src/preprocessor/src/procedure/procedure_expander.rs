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
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::procedure_ast::{
    Arg, BodyNode, MacroCallDef, ProcedureDef, ProcedureFile, Statement, TextPart,
};
use crate::procedure_parser::{ProcedureParseError, ProcedureParserService};
use parser_core::DiagramParser;
use puml_utils::LogLevel;

const DEFAULT_MACRO_DEPTH: usize = 10;

#[derive(Debug, thiserror::Error)]
pub enum ProcedureExpandError {
    #[error("Failed to parse procedure file {file}: {error}")]
    ParseFailed {
        file: Rc<PathBuf>,
        #[source]
        error: ProcedureParseError,
    },

    #[error("macro not defined: {0}")]
    MacroNotDefined(String),

    #[error("macro {name} expects {expected} args but got {actual}")]
    ArgumentMismatch {
        name: String,
        expected: usize,
        actual: usize,
    },

    #[error("variable not defined: {name}")]
    UnknownVariable { name: String },

    #[error("recursive macro detected: {chain:?} -> {name}")]
    RecursiveMacro { chain: Vec<String>, name: String },

    #[error("maximum macro expansion depth exceeded")]
    MaxDepthExceeded,
}

#[derive(Default)]
pub struct ProcedureExpander {
    parser: ProcedureParserService,
    procedures: HashMap<String, ProcedureDef>,
    max_depth: usize,
}

impl ProcedureExpander {
    pub fn new() -> Self {
        Self {
            max_depth: DEFAULT_MACRO_DEPTH,
            ..Default::default()
        }
    }

    pub fn expand(
        &mut self,
        path: &Rc<PathBuf>,
        content: &str,
        log_level: LogLevel,
    ) -> Result<String, ProcedureExpandError> {
        let mut out = String::new();
        let mut stack = Vec::new();

        self.procedures.clear();
        let procedure_ast = self.load_procedures(path, content, log_level)?;
        debug!("Loaded procedures from {:?}: {:#?}", path, procedure_ast);

        for stmt in &procedure_ast.stmts {
            match stmt {
                Statement::Text(t) => {
                    out.push_str(t);
                    out.push('\n');
                }
                Statement::MacroCall(call) => {
                    let mut params = HashMap::new();
                    out.push_str(&self.expand_macro(call, &mut params, &mut stack, 0)?);
                }
                Statement::Procedure(_) => {}
            }
        }

        Ok(out)
    }

    fn load_procedures(
        &mut self,
        path: &Rc<PathBuf>,
        content: &str,
        log_level: LogLevel,
    ) -> Result<ProcedureFile, ProcedureExpandError> {
        let procedure_ast = self
            .parser
            .parse_file(path, content, log_level)
            .map_err(|e| ProcedureExpandError::ParseFailed {
                file: Rc::clone(path),
                error: e,
            })?;

        for stmt in &procedure_ast.stmts {
            if let Statement::Procedure(p) = stmt {
                self.procedures.insert(p.name.clone(), p.clone());
            }
        }

        Ok(procedure_ast)
    }

    fn expand_macro(
        &self,
        call: &MacroCallDef,
        parent_params: &mut HashMap<String, String>,
        stack: &mut Vec<String>,
        depth: usize,
    ) -> Result<String, ProcedureExpandError> {
        if depth > self.max_depth {
            return Err(ProcedureExpandError::MaxDepthExceeded);
        }

        if stack.contains(&call.name) {
            return Err(ProcedureExpandError::RecursiveMacro {
                chain: stack.clone(),
                name: call.name.clone(),
            });
        }

        let proc = match self.resolve_proc(call, stack)? {
            Some(proc) => proc,
            None => {
                // Not found: keep the call text, but substitute any known $variables in arguments.
                return Ok(self.render_unknown_call(call, parent_params));
            }
        };

        if proc.params.len() != call.args.len() {
            return Err(ProcedureExpandError::ArgumentMismatch {
                name: call.name.clone(),
                expected: proc.params.len(),
                actual: call.args.len(),
            });
        }

        stack.push(call.name.clone());
        let mut new_params = self.build_params(proc, call, parent_params)?;
        let result = self.expand_body(&proc.body, &mut new_params, stack, depth + 1)?;
        stack.pop();

        Ok(result)
    }

    fn resolve_proc(
        &self,
        call: &MacroCallDef,
        stack: &[String],
    ) -> Result<Option<&ProcedureDef>, ProcedureExpandError> {
        let proc_opt = self.procedures.get(&call.name);
        if call.name.starts_with('$') {
            return proc_opt
                .ok_or_else(|| ProcedureExpandError::MacroNotDefined(call.name.clone()))
                .map(Some);
        }

        if stack.is_empty() {
            return proc_opt
                .ok_or_else(|| ProcedureExpandError::MacroNotDefined(call.name.clone()))
                .map(Some);
        }

        Ok(proc_opt)
    }

    // Literal render for unresolved non-$ calls inside a macro body (stack non-empty).
    fn render_unknown_call(
        &self,
        call: &MacroCallDef,
        parent_params: &HashMap<String, String>,
    ) -> String {
        let args_text = call
            .args
            .iter()
            .map(|arg| match arg {
                Arg::Variable(v) => parent_params
                    .get(v)
                    .cloned()
                    .unwrap_or_else(|| v.to_string()),
                Arg::String(s) => {
                    if s.starts_with('$') {
                        parent_params
                            .get(s)
                            .map(|value| format!("\"{}\"", value))
                            .unwrap_or_else(|| format!("\"{}\"", s))
                    } else {
                        format!("\"{}\"", s)
                    }
                }
                Arg::Number(n) => n.to_string(),
                Arg::Identifier(id) => id.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}({})\n", call.name, args_text)
    }

    fn build_params(
        &self,
        proc: &ProcedureDef,
        call: &MacroCallDef,
        parent_params: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, ProcedureExpandError> {
        let mut new_params = HashMap::new();
        for (param, arg) in proc.params.iter().zip(&call.args) {
            let value = match arg {
                Arg::Variable(v) => parent_params
                    .get(v)
                    .cloned()
                    .ok_or_else(|| ProcedureExpandError::UnknownVariable { name: v.clone() })?,
                Arg::String(s) => s.clone(),
                Arg::Number(n) => n.to_string(),
                Arg::Identifier(id) => id.clone(),
            };
            new_params.insert(param.clone(), value);
        }

        Ok(new_params)
    }

    fn expand_body(
        &self,
        body: &[BodyNode],
        params: &mut HashMap<String, String>,
        stack: &mut Vec<String>,
        depth: usize,
    ) -> Result<String, ProcedureExpandError> {
        let mut out = String::new();

        for node in body {
            match node {
                BodyNode::MacroCall(call) => {
                    out.push_str(&self.expand_macro(call, params, stack, depth)?);
                }
                BodyNode::Text(parts) => {
                    for part in parts {
                        match part {
                            TextPart::Literal(s) => {
                                out.push_str(s);
                            }
                            TextPart::Variable(v) => {
                                if let Some(val) = params.get(v) {
                                    out.push_str(val);
                                } else {
                                    out.push_str(v);
                                }
                            }
                        }
                    }
                    out.push('\n');
                }
            }
        }

        Ok(out)
    }
}
