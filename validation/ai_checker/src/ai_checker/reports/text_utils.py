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
"""Shared text helpers for report renderers (slugs, severity, markdown→HTML)."""

import html
import re


def normalize_filename(text: str) -> str:
    """Convert text to a filesystem-safe filename slug.

    Lowercase, spaces/underscores to hyphens, unsafe characters removed,
    collapsed and trimmed hyphens.
    """
    slug = text.lower()
    slug = re.sub(r"[\s_]+", "-", slug)
    slug = re.sub(r"[^a-z0-9-]", "", slug)
    slug = slug.strip("-")
    slug = re.sub(r"-+", "-", slug)
    return slug


def extract_severity(finding: str) -> str:
    """Extract a severity CSS class from finding text.

    The severity word (``Major`` / ``Minor``) may be wrapped in markdown
    (``**...**``) or HTML (``<b>`` / ``<strong>``, escaped or not) and may be
    followed by a colon, an en-dash summary (``Minor – …:``) or other text.
    Strip any leading emphasis marker, then read the first word.

    Returns 'major', 'minor', or '' when no severity is found.
    """
    # Drop a leading emphasis marker: ** , <b>, <strong> or their escaped forms.
    stripped = re.sub(
        r"^\s*(?:\*\*|<(?:b|strong)>|&lt;(?:b|strong)&gt;)\s*",
        "",
        finding,
        flags=re.IGNORECASE,
    )
    match = re.match(r"(Major|Minor)\b", stripped, re.IGNORECASE)
    if match:
        return match.group(1).lower()
    return ""


# Inline HTML tags the model is allowed to emit in findings/suggestions. After
# escaping the whole string (so arbitrary/unsafe HTML cannot pass through),
# only these bare tags are re-enabled — no attributes are un-escaped, so this
# stays XSS-safe.
_ALLOWED_INLINE_TAGS = ("b", "strong", "i", "em", "u", "code")
_ESCAPED_TAG_RE = re.compile(
    r"&lt;(/?(?:" + "|".join(_ALLOWED_INLINE_TAGS) + r"))&gt;",
    re.IGNORECASE,
)


def markdown_to_html(text: str) -> str:
    """Convert markdown emphasis to HTML, escaping everything else.

    The model sometimes emits a small set of inline HTML tags (e.g. ``<b>``)
    directly. The whole string is escaped first (preventing HTML injection),
    then markdown ``**bold**`` / ``*italic*`` are translated and the allowlisted
    inline tags are re-enabled so they render instead of showing as literal
    ``&lt;b&gt;`` text.
    """
    text = html.escape(text)
    text = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", text)
    text = re.sub(r"\*([^*]+)\*", r"<em>\1</em>", text)
    text = _ESCAPED_TAG_RE.sub(r"<\1>", text)
    text = text.replace("\n", "<br>\n")
    return text


def text_to_html(text: str) -> str:
    """Escape plain text and convert line breaks to ``<br>`` tags."""
    escaped = html.escape(text)
    return escaped.replace("\n", "<br>\n")


_RST_SPECIAL = re.compile(r"^([=\-`:.'\"~^_*+#])")


def strip_markup(text: str) -> str:
    """Reduce markdown/HTML formatting to plain text for reST output.

    reST is plain-text; we drop bold/italic markers and HTML tags rather than
    translate them, keeping the content readable and valid.
    """
    text = re.sub(r"<[^>]+>", "", text)  # drop HTML tags
    text = re.sub(r"\*\*([^*]+)\*\*", r"\1", text)  # **bold**
    text = re.sub(r"\*([^*]+)\*", r"\1", text)  # *italic*
    return text.strip()
