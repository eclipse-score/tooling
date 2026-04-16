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
"""
Result formatter for TRLC AI Checker analysis results.

This module provides formatting and output functionality for analysis results
in various formats (stdout, JSON, HTML).
"""

import html
import os
import re
import subprocess
from datetime import datetime
from typing import Any, Dict, Optional

from ai_checker.analysis_models import AnalysisResults
from ai_checker.guidelines_reader import GuidelinesReader


class ResultFormatter:
    """
    Handles formatting and output of analysis results in multiple formats.
    """

    @staticmethod
    def _get_git_hash() -> str:
        """Get the current git commit hash.

        First checks the BUILD_EMBED_LABEL / STABLE_GIT_COMMIT stamp variables
        injected by Bazel --workspace_status_command.  Falls back to running
        ``git rev-parse HEAD`` in the source tree, and finally returns
        'Unknown' if neither is available (e.g. inside a fully hermetised
        Bazel action without network/git access).

        Returns:
            Git commit hash (8 chars) or 'Unknown'
        """
        # Prefer Bazel workspace-status stamp variables (set by --workspace_status_command)
        for env_var in ("STABLE_GIT_COMMIT", "BUILD_EMBED_LABEL", "GIT_COMMIT"):
            value = os.environ.get(env_var, "").strip()
            if value:
                return value[:8]

        try:
            # Fall back to running git directly (works for local CLI invocations)
            result = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                capture_output=True,
                text=True,
                timeout=5,
                cwd=os.path.dirname(os.path.abspath(__file__)),
            )
            if result.returncode == 0:
                return result.stdout.strip()[:8]
        except (FileNotFoundError, subprocess.TimeoutExpired, OSError):
            pass
        return "Unknown"

    @staticmethod
    def _get_timestamp() -> str:
        """Get the current timestamp.

        Returns:
            ISO format timestamp string
        """
        return datetime.now().isoformat(timespec="seconds")

    @staticmethod
    def _normalize_filename(text: str) -> str:
        """Convert text to filesystem-safe filename slug.

        Args:
            text: Text to normalize

        Returns:
            Normalized string (lowercase, spaces to hyphens, unsafe chars removed)
        """
        # Convert to lowercase
        slug = text.lower()
        # Replace spaces and underscores with hyphens
        slug = re.sub(r"[\s_]+", "-", slug)
        # Remove unsafe characters (keep alphanumeric and hyphens)
        slug = re.sub(r"[^a-z0-9-]", "", slug)
        # Remove leading/trailing hyphens
        slug = slug.strip("-")
        # Collapse multiple hyphens
        slug = re.sub(r"-+", "-", slug)
        return slug

    @staticmethod
    def _extract_severity(finding: str) -> str:
        """Extract severity level from finding text.

        Args:
            finding: Finding text that may start with Major:, Minor:, **Major**:, <b>Major:</b>, or <strong>Major:</strong>

        Returns:
            CSS class name: 'major', 'minor', or empty string if no severity found
        """
        # Check for plain text format: Major: or Minor:
        match = re.match(r"^(Major|Minor):\s", finding, re.IGNORECASE)
        if match:
            return match.group(1).lower()
        # Check for markdown format: **Major**: or **Minor**:
        match = re.match(r"^\*\*(Major|Minor)\*\*:", finding, re.IGNORECASE)
        if match:
            return match.group(1).lower()
        # Check for HTML format: <b>Major:</b> or <b>Minor:</b> or <strong>Major:</strong>
        match = re.match(
            r"^<(?:b|strong)>(Major|Minor):</(?:b|strong)>", finding, re.IGNORECASE
        )
        if match:
            return match.group(1).lower()
        # Check for escaped HTML: &lt;b&gt;Major:&lt;/b&gt; or &lt;strong&gt;Minor:&lt;/strong&gt;
        match = re.match(
            r"^&lt;(?:b|strong)&gt;(Major|Minor):&lt;/(?:b|strong)&gt;",
            finding,
            re.IGNORECASE,
        )
        if match:
            return match.group(1).lower()
        return ""

    @staticmethod
    def _markdown_to_html(text: str) -> str:
        """Convert markdown formatting to HTML while preserving existing HTML tags.

        Args:
            text: Text with markdown and/or HTML formatting

        Returns:
            HTML-formatted string with markdown converted and HTML preserved
        """
        # First escape HTML to protect it, but mark HTML tags specially
        # Replace HTML tags with placeholders
        html_tags = []

        def save_html_tag(match):
            html_tags.append(match.group(0))
            return f"__HTML_TAG_{len(html_tags) - 1}__"

        # Save HTML tags
        text = re.sub(r"<[^>]+>", save_html_tag, text)

        # Now escape any remaining HTML special characters
        text = html.escape(text)

        # Convert markdown formatting
        # Bold: **text** to <strong>text</strong>
        text = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", text)
        # Italic: *text* to <em>text</em>
        text = re.sub(r"\*([^*]+)\*", r"<em>\1</em>", text)

        # Restore HTML tags
        for i, tag in enumerate(html_tags):
            text = text.replace(f"__HTML_TAG_{i}__", tag)

        # Convert line breaks to <br> tags
        text = text.replace("\n", "<br>\n")

        return text

    @staticmethod
    def _text_to_html(text: str) -> str:
        """Convert plain text to HTML with proper line breaks and escaping.

        Args:
            text: Plain text string

        Returns:
            HTML-formatted string with line breaks converted to <br> tags
        """
        # Escape HTML special characters
        escaped = html.escape(text)
        # Convert line breaks to <br> tags
        return escaped.replace("\n", "<br>\n")

    def __init__(
        self,
        analysis_results: AnalysisResults,
        model_name: Optional[str] = None,
        guidelines_reader: Optional[GuidelinesReader] = None,
        guidelines_output_dir: Optional[str] = None,
        original_requirements: Optional[Dict[str, Dict[str, Any]]] = None,
    ):
        """
        Initialize the formatter with analysis results.

        Args:
            analysis_results: AnalysisResults object containing analyses
            model_name: Name of the AI model used for analysis
            guidelines_reader: GuidelinesReader object containing all guidelines
            guidelines_output_dir: Optional directory path for writing guideline files
            original_requirements: Original requirement data as dict {id: {metadata}}
        """
        self.results = analysis_results
        self.model_name = model_name or "Unknown"
        self.guidelines_reader = guidelines_reader
        self.guidelines_output_dir = guidelines_output_dir
        self.git_hash = self._get_git_hash()
        self.timestamp = self._get_timestamp()

        # Create lookup map for original requirement descriptions
        self.original_descriptions = {}
        if original_requirements:
            for req_id, req_data in original_requirements.items():
                self.original_descriptions[req_id] = req_data.get("description", "")

    def output(self, file_path: Optional[str] = None) -> None:
        """
        Output results based on file path extension or to stdout.

        Args:
            file_path: Optional path to output file. If None, prints JSON to stdout.
                      Extension determines format: .html for HTML, otherwise JSON.
        """
        if file_path is None:
            self._print_to_stdout()
        else:
            # Create parent directories if they don't exist
            parent = os.path.dirname(file_path)
            if parent:
                os.makedirs(parent, exist_ok=True)

            extension = os.path.splitext(file_path)[1].lower()

            if extension == ".html":
                self._write_html(file_path)
            else:
                self._write_json(file_path)

            print(f"Analysis results written to {file_path}")

    def _print_to_stdout(self) -> None:
        """Print results as JSON to stdout."""
        output = self.results.model_dump_json(indent=2)
        print(output)

    def _write_json(self, path: str) -> None:
        """
        Write results as JSON file.

        Args:
            path: File path for output JSON file
        """
        with open(path, "w", encoding="utf-8") as f:
            f.write(self.results.model_dump_json(indent=2))

    def _write_html(self, path: str) -> None:
        """
        Write results as HTML report with guideline subpages.

        Args:
            path: File path for output HTML file
        """

        # Generate main report
        html_content = self._generate_html_report(path)
        with open(path, "w", encoding="utf-8") as f:
            f.write(html_content)

        # Generate guideline subpages
        if self.guidelines_reader:
            self._generate_guideline_pages(path)

    def _generate_html_report(self, main_report_path: Optional[str] = None) -> str:
        """
        Generate HTML report from analysis results.

        Args:
            main_report_path: Optional path to main report file for computing relative links

        Returns:
            HTML string containing formatted report
        """
        # Calculate summary statistics
        total_requirements = len(self.results.analyses)
        avg_score = (
            sum(a.score for a in self.results.analyses) / total_requirements
            if total_requirements > 0
            else 0
        )

        # Escape all untrusted values before interpolating into HTML
        safe_git_hash = html.escape(self.git_hash)
        safe_timestamp = html.escape(self.timestamp)
        safe_model_name = html.escape(self.model_name)

        doc = f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Requirements Analysis Report</title>
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f5f5f5;
        }}
        .header {{
            background: linear-gradient(135deg, #031E49 0%, #2D4046 100%);
            color: #FFFDFE;
            padding: 30px;
            border-radius: 10px;
            margin-bottom: 30px;
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        }}
        .header h1 {{
            margin: 0 0 10px 0;
        }}
        .summary {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }}
        .summary-card {{
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            text-align: center;
        }}
        .summary-card h3 {{
            margin: 0;
            color: #666;
            font-size: 14px;
            text-transform: uppercase;
        }}
        .summary-card .value {{
            font-size: 32px;
            font-weight: bold;
            color: #4599FE;
            margin-top: 10px;
        }}
        .requirement {{
            background: white;
            margin-bottom: 20px;
            padding: 25px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .requirement-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 15px;
            padding-bottom: 15px;
            border-bottom: 2px solid #f0f0f0;
        }}
        .requirement-id {{
            font-size: 18px;
            font-weight: bold;
            color: #333;
        }}
        .score {{
            font-size: 24px;
            font-weight: bold;
            padding: 5px 15px;
            border-radius: 20px;
            background: linear-gradient(135deg, #4599FE 0%, #031E49 100%);
            color: #FFFDFE;
        }}
        .score.low {{
            background: linear-gradient(135deg, #EE0405 0%, #2D4046 100%);
        }}
        .score.medium {{
            background: linear-gradient(135deg, #B8CAD1 0%, #2D4046 100%);
        }}
        .score.high {{
            background: linear-gradient(135deg, #4599FE 0%, #031E49 100%);
        }}
        .description {{
            margin-bottom: 15px;
            color: #555;
        }}
        .section {{
            margin-bottom: 15px;
        }}
        .section h4 {{
            margin: 0 0 10px 0;
            color: #4599FE;
            font-size: 14px;
            text-transform: uppercase;
        }}
        .findings, .suggestions {{
            list-style: none;
            padding: 0;
        }}
        .findings li, .suggestions li {{
            padding: 8px 0 8px 30px;
            position: relative;
            color: #555;
        }}
        .findings li:before {{
            position: absolute;
            left: 0;
            font-size: 20px;
            content: "⚠";
            color: #EE0405;
        }}
        .findings li.major:before {{
            content: "🔴";
            color: inherit;
        }}
        .findings li.minor:before {{
            content: "⚠️";
            color: inherit;
        }}
        .suggestions li:before {{
            content: "💡";
            position: absolute;
            left: 0;
        }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Requirements Analysis Report</h1>
        <p>Comprehensive analysis of requirements against engineering guidelines</p>
        <div style="font-size: 12px; margin-top: 15px; opacity: 0.9;">
            <p>Hash: {safe_git_hash}</p>
            <p>Timestamp: {safe_timestamp}</p>
        </div>
    </div>

    <div class="summary">
        <div class="summary-card">
            <h3>Total Requirements</h3>
            <div class="value">{total_requirements}</div>
        </div>
        <div class="summary-card">
            <h3>Average Score</h3>
            <div class="value">{avg_score:.1f}/10</div>
        </div>
        <div class="summary-card">
            <h3>AI Model Used</h3>
            <div class="value" style="font-size: 18px;">{safe_model_name}</div>
        </div>
    </div>

    <div class="summary" style="margin-top: 20px;">
        <div class="summary-card" style="grid-column: 1 / -1;">
            <h3>Guidelines Used</h3>
            <div style="margin-top: 10px; text-align: left;">
                <ul style="list-style: none; padding: 0; margin: 0;">
{self._generate_guidelines_links(main_report_path)}
                </ul>
            </div>
        </div>
    </div>

    <div class="requirements">
"""

        # Add individual requirement sections
        for analysis in self.results.analyses:
            score_class = (
                "high"
                if analysis.score >= 8
                else "medium"
                if analysis.score >= 5
                else "low"
            )

            # Use original full description if available, otherwise use
            # AI's description
            description = self.original_descriptions.get(
                analysis.requirement_id, analysis.description
            )
            # Escape requirement_id: it comes from user-supplied TRLC files
            safe_req_id = html.escape(analysis.requirement_id)

            doc += f"""
        <div class="requirement">
            <div class="requirement-header">
                <div class="requirement-id">{safe_req_id}</div>
                <div class="score {score_class}">{analysis.score:.1f}/10</div>
            </div>

            <div class="description">
                <strong>Description:</strong> {self._text_to_html(description)}
            </div>
"""

            if analysis.findings:
                doc += """
            <div class="section">
                <h4>Findings</h4>
                <ul class="findings">
"""
                for finding in analysis.findings:
                    severity_class = self._extract_severity(finding)
                    formatted_finding = self._markdown_to_html(finding)
                    doc += f'                    <li class="{severity_class}">{formatted_finding}</li>\n'
                doc += """                </ul>
            </div>
"""

            if analysis.suggestions:
                doc += """
            <div class="section">
                <h4>Suggestions</h4>
                <ul class="suggestions">
"""
                for suggestion in analysis.suggestions:
                    doc += f"                    <li>{self._markdown_to_html(suggestion)}</li>\n"
                doc += """                </ul>
            </div>
"""

            doc += """        </div>
"""

        doc += """    </div>
</body>
</html>
"""
        return doc

    def _generate_guidelines_links(self, main_report_path: Optional[str] = None) -> str:
        """Generate HTML for guidelines list with links to subpages.

        Args:
            main_report_path: Path to main report (used to compute relative paths)

        Returns:
            HTML string with list items for guidelines
        """
        if not self.guidelines_reader or not self.guidelines_reader.guidelines:
            return (
                '                    <li style="padding: 5px 0; '
                'color: #999;">No guidelines specified</li>'
            )

        # Compute output directory for guidelines (same logic as _generate_guideline_pages)
        if main_report_path:
            report_dir = os.path.dirname(main_report_path)
            if self.guidelines_output_dir:
                output_dir = self.guidelines_output_dir
            else:
                output_dir = os.path.join(report_dir, "guidelines")

            # Compute relative path from report directory to guidelines directory
            try:
                relative_base_str = os.path.relpath(output_dir, report_dir)
            except ValueError:
                # If not relative, use absolute path
                relative_base_str = output_dir
        else:
            # Fallback to hardcoded path if no main_report_path provided
            relative_base_str = "guidelines"

        links = []
        for guideline_name in sorted(self.guidelines_reader.guidelines.keys()):
            # Normalize guideline name for filename
            slug = self._normalize_filename(guideline_name)
            links.append(
                f'                    <li style="padding: 5px 0;">'
                f'<a href="{relative_base_str}/guideline_{slug}.md" '
                f'style="color: #4599FE; text-decoration: none;">'
                f"📋 {guideline_name}</a></li>"
            )
        return "\n".join(links)

    def _generate_guideline_pages(self, main_report_path: str) -> None:
        """Generate markdown files for each guideline.

        Args:
            main_report_path: Path to main report (used to determine output directory)
        """
        if not self.guidelines_reader:
            return

        # Use guidelines subdirectory in the same parent directory as main report
        if self.guidelines_output_dir:
            output_dir = self.guidelines_output_dir
        else:
            output_dir = os.path.join(os.path.dirname(main_report_path), "guidelines")

        os.makedirs(output_dir, exist_ok=True)

        for (
            guideline_name,
            guideline_content,
        ) in self.guidelines_reader.guidelines.items():
            # Normalize guideline name for filename
            slug = self._normalize_filename(guideline_name)
            page_path = os.path.join(output_dir, f"guideline_{slug}.md")
            with open(page_path, "w", encoding="utf-8") as f:
                f.write(f"# {guideline_name}\n\n")
                f.write(guideline_content)
