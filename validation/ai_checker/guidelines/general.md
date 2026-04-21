<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

- Markdown Expressions (Link: "[label](http://example.com)")
- RST Expressions (Link: ".. _a link: https://domain.invalid/" or `a link`_ )
- Expression which are provided as comment or via `` as fully defined.

Do a requirements review of each single requirement:
- Review the requirements according to the guidelines, for the sentence template also take into consideration optional and mandatory parts of a sentence.
- Include the requirement type in the analysis.
- Accept expressions as defined if in doubt

Do not:
- analyze the hierarchy of the requirements or relations between them (e.g. stated via attribute parent)
- mention any findings of fully defined items
- take the sentence template to strict

Score the requirement from 0-10 where:
- 0-3: Critical issues, requirement needs major rework
- 4-6: Moderate issues, improvement needed
- 7-8: Good quality with minor improvements possible
- 9-10: Excellent quality, meets professional standards

Analysis Result:
- Provide specific, actionable findings and refer it to a specific keyword from the  document. (e.g. *Vague* Requirement contains ....)
- Only point out findings, don´t mention if something is particularly well defined
- Categorize findings in major and minor
- Provide an overall scoring for the requirement
- If required use HTML formatting
