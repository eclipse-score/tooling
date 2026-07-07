/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

namespace ara
{
namespace core
{
struct InstanceSpecifier
{
};
}  // namespace core
}  // namespace ara

class DiagnosticJobCollectionBuilder
{
  public:
    template <typename DiagnosticJobType, typename... DiagnosticJobConstructorArgumentTypes>
    void With(ara::core::InstanceSpecifier, DiagnosticJobConstructorArgumentTypes...);

    template <typename DiagnosticJobType, typename... DiagnosticJobConstructorArgumentTypes>
    DiagnosticJobType Build(::ara::core::InstanceSpecifier, DiagnosticJobConstructorArgumentTypes...);
};
