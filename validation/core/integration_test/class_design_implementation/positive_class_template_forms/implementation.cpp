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

namespace amp
{
template <typename T, typename U>
class ordinary_template
{
public:
    T value() const;
};

template <typename T, typename U>
class quoted_template_alias
{
public:
    T value() const;
};

template <typename T, typename U>
class quoted_template_no_alias
{
public:
    T value() const;
};

template <typename... Spec>
class Proxy
{
};
}  // namespace amp
