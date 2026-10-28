..
   # *******************************************************************************
   # Copyright (c) 2026 Contributors to the Eclipse Foundation
   #
   # See the NOTICE file(s) distributed with this work for additional
   # information regarding copyright ownership.
   #
   # This program and the accompanying materials are made available under the
   # terms of the Apache License Version 2.0 which is available at
   # https://www.apache.org/licenses/LICENSE-2.0
   #
   # SPDX-License-Identifier: Apache-2.0
   # *******************************************************************************

Read operation returns a stable value
=====================================

Test Goal
---------
Show that the read operation of the numeric value management interface
returns the expected ``uint8_t`` value, independent of how often and
through which kind of reference it is called.

Test Focus
----------
- interface behavior of ``Foo::GetNumber``
- repeated reads through mutable and const references

Test Steps
----------

Step 1: First read
~~~~~~~~~~~~~~~~~~
:Precondition: A default-constructed ``Foo`` instance exists.
:Action: Call ``GetNumber`` once.
:Expected: The returned value is 42.

Step 2: Repeated read through a const reference
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
:Action: Call ``GetNumber`` again through a const reference to the same instance.
:Expected: The returned value is still 42.
