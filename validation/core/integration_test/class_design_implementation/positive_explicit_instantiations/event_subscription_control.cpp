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

#include "event_subscription_control.h"

namespace score {
namespace mw {
namespace com {
namespace impl {
namespace lola {

// Regression: the header definition and this explicit instantiation share the
// same qualified class name, but only the header definition must be collected.
template class EventSubscriptionControl<concurrency::AtomicIndirectorReal>;

} // namespace lola
} // namespace impl
} // namespace com
} // namespace mw
} // namespace score
