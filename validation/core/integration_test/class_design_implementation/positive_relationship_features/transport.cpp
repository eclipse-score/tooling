/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

class Base
{
};

struct Owned
{
};

struct Borrowed
{
};

struct Result
{
};

struct Context
{
};

struct Worker : Base
{
    Owned owned;
    Borrowed* borrowed;

    Result make_result(const Context& context);
};
