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

Skills Setup
============

``score_tooling`` ships a set of shared Copilot "skills" (``score-*`` under
``.github/skills``) that document how to use ``rules_score`` (architecture,
requirements, safety analysis, testing). Downstream repositories can pull
these into their own ``.github/skills`` directory so the same guidance is
available to Copilot locally.

- ``bazel run //:sync_skills`` — copies score_tooling's current ``score-*``
  skills into the local ``.github/skills`` directory, overwriting outdated
  copies and removing skills that score_tooling no longer ships.
- ``bazel test //:sync_skills.check`` — fails if the committed skills are
  missing, outdated, or stale relative to the ``score_tooling`` version in
  use. Wire this into CI so upstream skill updates are caught automatically.
