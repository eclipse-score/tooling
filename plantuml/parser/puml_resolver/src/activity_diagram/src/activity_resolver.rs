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

use crate::activity_logic::ActivityDiagram;
use activity_parser::RawActivityDiagram;
use resolver_traits::DiagramResolver;

#[derive(Debug, thiserror::Error)]
pub enum ActivityResolverError {
    #[error("activity resolver logic is not implemented yet")]
    NotImplemented,
}

#[derive(Debug, Default)]
pub struct ActivityResolver;

impl ActivityResolver {
    pub fn new() -> Self {
        Self
    }
}

impl DiagramResolver for ActivityResolver {
    type Document = RawActivityDiagram;
    type Output = ActivityDiagram;
    type Error = ActivityResolverError;

    fn resolve(&mut self, _document: &Self::Document) -> Result<Self::Output, Self::Error> {
        Err(ActivityResolverError::NotImplemented)
    }
}
