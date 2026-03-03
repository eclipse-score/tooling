"""Shared utilities for SBOM formatters."""

import re


def _normalize_spdx_license(expr: str) -> str:
    """Normalize SPDX boolean operators to uppercase as required by the spec.

    dash-license-scan returns lowercase operators (e.g. 'Apache-2.0 or MIT').
    SPDX 2.3 requires uppercase OR/AND/WITH (Appendix IV).
    Uses space-delimited substitution to avoid modifying license identifiers
    that contain 'or'/'and' as substrings (e.g. GPL-2.0-or-later).
    """
    expr = re.sub(r" or ", " OR ", expr, flags=re.IGNORECASE)
    expr = re.sub(r" and ", " AND ", expr, flags=re.IGNORECASE)
    expr = re.sub(r" with ", " WITH ", expr, flags=re.IGNORECASE)
    return expr
