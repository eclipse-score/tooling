/****************************************************************************
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
 ****************************************************************************/

#ifndef POSITIVE_EXPLICIT_INSTANTIATIONS_EVENT_SUBSCRIPTION_CONTROL_H
#define POSITIVE_EXPLICIT_INSTANTIATIONS_EVENT_SUBSCRIPTION_CONTROL_H

namespace score {
namespace concurrency {

template <class T>
class AtomicIndirectorReal {};

} // namespace concurrency

namespace mw {
namespace com {
namespace impl {
namespace lola {

// Regression: a defaulted template-template parameter and final must not make
// the explicit instantiation in the source file a second class definition.
template <template <class> class AtomicIndirectorType = concurrency::AtomicIndirectorReal>
class EventSubscriptionControl final {};

} // namespace lola
} // namespace impl
} // namespace com
} // namespace mw
} // namespace score

#endif // POSITIVE_EXPLICIT_INSTANTIATIONS_EVENT_SUBSCRIPTION_CONTROL_H
