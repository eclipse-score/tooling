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

// Covers the supported and unsupported forms of the `noexcept` specifier:
//  - a bare `noexcept` on a regular method is recognized.
//  - `noexcept(expr)` (conditional) is not modeled and must not be reported.
//  - `noexcept` written inside a lambda in the method body must not leak onto
//    the enclosing method.
//  - an implicit/defaulted destructor's compiler-computed exception
//    specification must not be reported, even when a derived class in the
//    same translation unit forces it to be evaluated.
//  - a bare `noexcept` explicitly written on a defaulted destructor must
//    still be recognized.
class NoexceptSample {
public:
  void plainMethod();
  void explicitNoexceptMethod() noexcept;
  void conditionalNoexceptFalse() noexcept(false);
  void conditionalNoexceptTrue() noexcept(true);
  void methodWithNoexceptLambdaInBody() {
    auto lambda = []() noexcept { return 0; };
    (void)lambda;
  }
};

class ImplicitDestructorBase {
public:
  virtual ~ImplicitDestructorBase() = default;
  virtual void run() = 0;
};

class ExplicitNoexceptDestructorBase {
public:
  virtual ~ExplicitNoexceptDestructorBase() noexcept = default;
  virtual void run() = 0;
};

class ImplicitDestructorDerived : public ImplicitDestructorBase {
public:
  void run() override;
};

class ExplicitNoexceptDestructorDerived
    : public ExplicitNoexceptDestructorBase {
public:
  void run() override;
};
