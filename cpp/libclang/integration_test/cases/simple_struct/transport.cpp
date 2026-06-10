/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

typedef struct Manufacturer
{
    const char* name;
    int country_code;
} Manufacturer;

struct Engine
{
    int cylinders;
    double displacement;
    char manufacturer[32];

    const char* model;
    volatile int status;

    Manufacturer vendor;

    // TBD: check plantuml class parser can support.
    void (*start)(struct Engine*);
    int (*stop)(struct Engine*, int force);
};
