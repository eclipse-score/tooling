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

class ContainerBase {
public:
  virtual ~ContainerBase() = default;
};

template <typename T> T declval();

template <typename T>
auto is_maplike_container_impl(T value) -> decltype(value.begin());

// Regression test for a real parser crash: a base class specifier whose type
// depends on an uninstantiated template parameter, e.g. through the common
// decltype/SFINAE detection idiom, cannot be resolved to a concrete entity id
// without template instantiation. The parser must not panic on this; it
// should simply skip the unresolvable inheritance relationship.
template <typename T>
struct IsMaplikeContainer : decltype(is_maplike_container_impl(declval<T>())){};

// A normal, resolvable base class must still be parsed correctly even though
// the translation unit also contains the unresolvable template above.
class Container : public ContainerBase {};

// A concrete, non-template return type used purely so `decltype(make_widget())`
// below has something real to resolve to.
class Widget {
public:
  Widget();
};

Widget make_widget();

class ConcreteWidgetUser : public decltype(make_widget()){};
