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

///  Resolver trait for PlantUML diagrams
pub trait DiagramResolver {
    type Document;
    type Statement;
    type Output;
    type Error;

    fn visit_document(&mut self, document: &Self::Document) -> Result<Self::Output, Self::Error>;

    fn visit_statement(&mut self, _statement: &Self::Statement) -> Result<(), Self::Error> {
        Ok(())
    }
}
