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
"""prompt_toolkit split-pane UI for interactive manual analyses."""

from __future__ import annotations

import importlib
import inspect
import os
import shlex
import subprocess
import tempfile
import threading
from collections.abc import Sized

from manual_analysis.interactive_runner_runtime import _workspace_root
from manual_analysis.yaml_schema import AutomatedActionArg


class _SplitPaneUI:
    def __init__(self) -> None:
        self._history: list[str] = []
        self._history_lock = threading.Lock()
        self._follow_left_panel_output = True
        self._left_panel_vertical_scroll = 0

    def _history_text(self) -> str:
        with self._history_lock:
            return (
                "\n\n".join(self._history)
                if self._history
                else "No analysis output yet."
            )

    def _append_history(self, title: str, body: str, separator: str) -> None:
        entry = f"{separator}\n{title}\n{separator}"
        if body:
            entry = f"{entry}\n{body}"
        with self._history_lock:
            self._history.append(entry)

    def _append_to_latest_history(
        self,
        content: str,
        *,
        separate_from_header: bool = False,
    ) -> None:
        with self._history_lock:
            if not self._history:
                self._history.append(content)
                return

            separator = "\n" if separate_from_header else ""
            self._history[-1] = f"{self._history[-1]}{separator}{content}"

    def print_header(self, title: str) -> None:
        history_separator = "#" * 78
        self._append_history(title, "", history_separator)

    def show_text(self, title: str, content: str) -> None:
        history_separator = "=" * 78
        self._append_history(title, content, history_separator)

    def _cursor_position_for_line(self, text: str, line_index: int) -> int:
        if line_index <= 0:
            return 0

        lines = text.splitlines(keepends=True)
        if not lines:
            return 0

        clamped_line_index = min(line_index, len(lines) - 1)
        return sum(len(line) for line in lines[:clamped_line_index])

    def _set_left_panel_scroll(self, left_area) -> None:  # type: ignore[no-untyped-def]
        window = getattr(left_area, "window", None)
        buffer = getattr(left_area, "buffer", None)
        if window is None or not hasattr(window, "vertical_scroll"):
            return

        if self._follow_left_panel_output:
            window.vertical_scroll = left_area.text.count("\n")
            if buffer is not None and hasattr(buffer, "cursor_position"):
                buffer.cursor_position = len(left_area.text)
            return

        window.vertical_scroll = self._left_panel_vertical_scroll
        if buffer is not None and hasattr(buffer, "cursor_position"):
            buffer.cursor_position = self._cursor_position_for_line(
                left_area.text,
                self._left_panel_vertical_scroll,
            )

    def _store_left_panel_scroll_state(self, left_area) -> None:  # type: ignore[no-untyped-def]
        window = getattr(left_area, "window", None)
        if window is None or not hasattr(window, "vertical_scroll"):
            return

        current_scroll = int(getattr(window, "vertical_scroll", 0) or 0)
        self._left_panel_vertical_scroll = max(0, current_scroll)

        render_info = getattr(window, "render_info", None)
        if render_info is None:
            return

        content_height = getattr(render_info, "content_height", None)
        if content_height is None:
            ui_content = getattr(render_info, "ui_content", None)
            content_height = getattr(ui_content, "line_count", None)

        window_height = getattr(render_info, "window_height", None)
        if window_height is None:
            displayed_lines = getattr(render_info, "displayed_lines", None)
            if isinstance(displayed_lines, Sized):
                try:
                    window_height = len(displayed_lines)
                except TypeError:
                    window_height = None

        if not isinstance(content_height, int) or not isinstance(window_height, int):
            return

        max_scroll = max(0, content_height - window_height)
        self._follow_left_panel_output = current_scroll >= max_scroll

    def _refresh_left_panel(self, left_area, app) -> None:  # type: ignore[no-untyped-def]
        self._store_left_panel_scroll_state(left_area)
        left_area.text = self._history_text()
        self._set_left_panel_scroll(left_area)
        app.invalidate()

    def _run_editor(self, initial_text: str) -> str:
        editor = os.environ.get("VISUAL") or os.environ.get("EDITOR") or "vi"
        with tempfile.NamedTemporaryFile(mode="w+", suffix=".tmp", delete=False) as tmp:
            tmp.write(initial_text)
            tmp.flush()
            tmp_path = tmp.name

        try:
            subprocess.run(shlex.split(editor) + [tmp_path], check=True)
            with open(tmp_path, encoding="utf-8") as file_obj:
                return file_obj.read()
        finally:
            try:
                os.unlink(tmp_path)
            except OSError:
                pass

    def _prompt_text(
        self,
        title: str,
        instructions: str,
        multiline: bool,
        initial_text: str,
        options: list[str] | None = None,
        default_option: str | None = None,
    ) -> str:
        application_module = importlib.import_module("prompt_toolkit.application")
        key_binding_module = importlib.import_module("prompt_toolkit.key_binding")
        layout_module = importlib.import_module("prompt_toolkit.layout")
        dimension_module = importlib.import_module("prompt_toolkit.layout.dimension")
        shortcuts_module = importlib.import_module("prompt_toolkit.shortcuts")
        widgets_module = importlib.import_module("prompt_toolkit.widgets")

        Application = getattr(application_module, "Application")
        run_in_terminal = getattr(application_module, "run_in_terminal", None)
        KeyBindings = getattr(key_binding_module, "KeyBindings")
        HSplit = getattr(layout_module, "HSplit")
        Layout = getattr(layout_module, "Layout")
        VSplit = getattr(layout_module, "VSplit")
        Dimension = getattr(dimension_module, "Dimension")
        message_dialog = getattr(shortcuts_module, "message_dialog")
        Frame = getattr(widgets_module, "Frame")
        TextArea = getattr(widgets_module, "TextArea")

        def _get_keys_text() -> str:
            if layout.has_focus(left_area):
                return "Tab/Shift-Tab: switch panel | Up/Down/PgUp/PgDn: scroll"
            if layout.has_focus(instructions_area):
                return "Tab/Shift-Tab: switch panel | Up/Down/PgUp/PgDn: scroll"
            keys = "Tab/Shift-Tab: switch panel | Up/Down/PgUp/PgDn: scroll | Ctrl-S/F2: submit"
            if multiline:
                keys += " | F4: open editor"
            return keys

        while True:
            left_area = TextArea(
                text=self._history_text(),
                read_only=True,
                focusable=True,
                scrollbar=True,
                wrap_lines=False,
            )
            right_area = TextArea(
                text=initial_text,
                multiline=multiline,
                focusable=True,
                scrollbar=True,
                wrap_lines=True,
            )
            instructions_area = TextArea(
                text=instructions,
                read_only=True,
                focusable=True,
                scrollbar=True,
                wrap_lines=True,
            )
            status_area = TextArea(
                text="",
                read_only=True,
                focusable=False,
                wrap_lines=True,
            )
            key_bindings = KeyBindings()

            @key_bindings.add("tab")
            def _focus_next(event) -> None:  # type: ignore[no-untyped-def]
                if event.app.layout.has_focus(left_area):
                    event.app.layout.focus(instructions_area)
                elif event.app.layout.has_focus(instructions_area):
                    event.app.layout.focus(right_area)
                else:
                    event.app.layout.focus(left_area)
                status_area.text = _get_keys_text()

            @key_bindings.add("s-tab")
            def _focus_previous(event) -> None:  # type: ignore[no-untyped-def]
                if event.app.layout.has_focus(right_area):
                    event.app.layout.focus(instructions_area)
                elif event.app.layout.has_focus(instructions_area):
                    event.app.layout.focus(left_area)
                else:
                    event.app.layout.focus(right_area)
                status_area.text = _get_keys_text()

            @key_bindings.add("c-s")
            @key_bindings.add("f2")
            def _submit(event) -> None:  # type: ignore[no-untyped-def]
                event.app.exit(result=right_area.text)

            @key_bindings.add("f4")
            async def _open_editor(event) -> None:  # type: ignore[no-untyped-def]
                edited = right_area.text

                def _edit_in_terminal() -> None:
                    nonlocal edited
                    edited = self._run_editor(right_area.text)

                if callable(run_in_terminal):
                    run_in_terminal_result = run_in_terminal(_edit_in_terminal)
                    if inspect.isawaitable(run_in_terminal_result):
                        await run_in_terminal_result
                else:
                    _edit_in_terminal()

                right_area.text = edited
                event.app.layout.focus(right_area)
                status_area.text = _get_keys_text()

            @key_bindings.add("c-c")
            def _abort(event) -> None:  # type: ignore[no-untyped-def]
                raise KeyboardInterrupt

            layout = Layout(
                HSplit(
                    [
                        VSplit(
                            [
                                Frame(left_area, title="Analysis Progress"),
                                HSplit(
                                    [
                                        Frame(
                                            instructions_area,
                                            title="Instructions",
                                            height=Dimension(min=1, max=6),
                                        ),
                                        Frame(
                                            right_area,
                                            title=title,
                                            height=Dimension(weight=1),
                                        ),
                                    ],
                                    height=Dimension(weight=1),
                                ),
                            ],
                            height=Dimension(weight=1),
                        ),
                        Frame(
                            status_area,
                            title="Keys",
                            height=Dimension(min=3, max=3),
                        ),
                    ],
                    height=Dimension(weight=1),
                ),
                focused_element=right_area,
            )
            status_area.text = _get_keys_text()

            app = Application(
                layout=layout, key_bindings=key_bindings, full_screen=True
            )
            pre_run_callables = getattr(app, "pre_run_callables", None)
            if isinstance(pre_run_callables, list):
                pre_run_callables.append(lambda: self._set_left_panel_scroll(left_area))
            else:
                self._set_left_panel_scroll(left_area)
            submitted: str | None = app.run()
            self._store_left_panel_scroll_state(left_area)
            if submitted is None:
                raise KeyboardInterrupt
            submitted_text = str(submitted)

            value = submitted_text.strip() if not multiline else submitted_text.strip()
            if options:
                if value == "" and default_option is not None:
                    return default_option
                allowed = {option.lower(): option for option in options}
                selected = allowed.get(value.lower())
                if selected is not None:
                    return selected
                message_dialog(
                    title="Invalid input",
                    text=f"Expected one of: {', '.join(options)}",
                ).run()
                initial_text = submitted_text
                continue

            return value

        raise RuntimeError("Prompt loop exited unexpectedly")

    def prompt_choice(
        self,
        description: str,
        options: list[str],
        default_option: str | None = None,
    ) -> str:
        instructions = f"{description}\n\nAllowed answers: {', '.join(options)}"
        selected = self._prompt_text(
            title="Current Decision",
            instructions=instructions,
            multiline=False,
            initial_text=default_option or "",
            options=options,
            default_option=default_option,
        )
        self._append_history(
            "Decision Answer",
            f"{description}\nAnswer: {selected}",
            "=" * 78,
        )
        return selected

    def prompt_justification(
        self,
        prompt: str,
        default_text: str | None = None,
    ) -> str:
        justification = self._prompt_text(
            title="Justification",
            instructions=(
                f"{prompt}\n\n"
                "Provide a short rationale for your selected answer. "
                "Leave empty if no justification is needed."
            ),
            multiline=True,
            initial_text=default_text or "",
        )
        if justification:
            self._append_history("Justification", justification, "=" * 78)
        return justification

    def prompt_choice_with_justification(
        self,
        description: str,
        options: list[str],
        default_option: str | None = None,
        default_justification: str | None = None,
    ) -> tuple[str, str]:
        application_module = importlib.import_module("prompt_toolkit.application")
        key_binding_module = importlib.import_module("prompt_toolkit.key_binding")
        layout_module = importlib.import_module("prompt_toolkit.layout")
        dimension_module = importlib.import_module("prompt_toolkit.layout.dimension")
        shortcuts_module = importlib.import_module("prompt_toolkit.shortcuts")
        widgets_module = importlib.import_module("prompt_toolkit.widgets")

        Application = getattr(application_module, "Application")
        KeyBindings = getattr(key_binding_module, "KeyBindings")
        HSplit = getattr(layout_module, "HSplit")
        Layout = getattr(layout_module, "Layout")
        VSplit = getattr(layout_module, "VSplit")
        Dimension = getattr(dimension_module, "Dimension")
        message_dialog = getattr(shortcuts_module, "message_dialog")
        Frame = getattr(widgets_module, "Frame")
        TextArea = getattr(widgets_module, "TextArea")

        answer_text = default_option or ""
        justification_text = default_justification or ""
        while True:
            left_area = TextArea(
                text=self._history_text(),
                read_only=True,
                focusable=True,
                scrollbar=True,
                wrap_lines=False,
            )
            instructions_area = TextArea(
                text=(
                    f"{description}\n\n"
                    f"Allowed answers: {', '.join(options)}\n"
                    "Provide an optional justification in the field below."
                ),
                read_only=True,
                focusable=True,
                scrollbar=True,
                wrap_lines=True,
            )
            answer_area = TextArea(
                text=answer_text,
                multiline=False,
                focusable=True,
                scrollbar=False,
                wrap_lines=False,
            )
            justification_area = TextArea(
                text=justification_text,
                multiline=True,
                focusable=True,
                scrollbar=True,
                wrap_lines=True,
            )
            status_area = TextArea(
                text="",
                read_only=True,
                focusable=False,
                wrap_lines=True,
            )
            key_bindings = KeyBindings()

            focus_order = [
                left_area,
                instructions_area,
                answer_area,
                justification_area,
            ]

            def _focused_index() -> int:
                for index, field in enumerate(focus_order):
                    if layout.has_focus(field):
                        return index
                return 0

            def _status_text() -> str:
                index = _focused_index()
                if focus_order[index] is answer_area:
                    return (
                        "Tab/Shift-Tab: switch focus | Ctrl-S/F2: submit | Answer field"
                    )
                if focus_order[index] is justification_area:
                    return (
                        "Tab/Shift-Tab: switch focus | Ctrl-S/F2: submit | "
                        "Justification field"
                    )
                if focus_order[index] is left_area:
                    return (
                        "Tab/Shift-Tab: switch focus | Up/Down/PgUp/PgDn: "
                        "scroll history"
                    )
                return "Tab/Shift-Tab: switch focus | Ctrl-S/F2: submit"

            @key_bindings.add("tab")
            def _focus_next(event) -> None:  # type: ignore[no-untyped-def]
                index = _focused_index()
                event.app.layout.focus(focus_order[(index + 1) % len(focus_order)])
                status_area.text = _status_text()

            @key_bindings.add("s-tab")
            def _focus_previous(event) -> None:  # type: ignore[no-untyped-def]
                index = _focused_index()
                event.app.layout.focus(focus_order[(index - 1) % len(focus_order)])
                status_area.text = _status_text()

            @key_bindings.add("c-s")
            @key_bindings.add("f2")
            def _submit(event) -> None:  # type: ignore[no-untyped-def]
                event.app.exit(
                    result={
                        "answer": answer_area.text,
                        "justification": justification_area.text,
                    }
                )

            @key_bindings.add("c-c")
            def _abort(_event) -> None:  # type: ignore[no-untyped-def]
                raise KeyboardInterrupt

            layout = Layout(
                HSplit(
                    [
                        VSplit(
                            [
                                Frame(left_area, title="Analysis Progress"),
                                HSplit(
                                    [
                                        Frame(
                                            instructions_area,
                                            title="Instructions",
                                            height=Dimension(min=4, max=7),
                                        ),
                                        Frame(
                                            answer_area,
                                            title="Selected Answer",
                                            height=Dimension(min=3, max=3),
                                        ),
                                        Frame(
                                            justification_area,
                                            title="Justification (Optional)",
                                            height=Dimension(weight=1),
                                        ),
                                    ],
                                    height=Dimension(weight=1),
                                ),
                            ],
                            height=Dimension(weight=1),
                        ),
                        Frame(
                            status_area,
                            title="Keys",
                            height=Dimension(min=3, max=3),
                        ),
                    ],
                    height=Dimension(weight=1),
                ),
                focused_element=answer_area,
            )

            app = Application(
                layout=layout,
                key_bindings=key_bindings,
                full_screen=True,
            )
            pre_run_callables = getattr(app, "pre_run_callables", None)
            if isinstance(pre_run_callables, list):
                pre_run_callables.append(lambda: self._set_left_panel_scroll(left_area))
                pre_run_callables.append(
                    lambda: setattr(status_area, "text", _status_text())
                )
            else:
                self._set_left_panel_scroll(left_area)
                status_area.text = _status_text()

            submitted = app.run()
            self._store_left_panel_scroll_state(left_area)
            if not isinstance(submitted, dict):
                raise KeyboardInterrupt

            submitted_answer = str(submitted.get("answer", "")).strip()
            submitted_justification = str(submitted.get("justification", "")).strip()
            if submitted_answer == "" and default_option is not None:
                selected = default_option
            else:
                allowed = {value.lower(): value for value in options}
                selected = allowed.get(submitted_answer.lower())
                if selected is None:
                    message_dialog(
                        title="Invalid input",
                        text=f"Expected one of: {', '.join(options)}",
                    ).run()
                    answer_text = submitted_answer
                    justification_text = submitted_justification
                    continue

            self._append_history(
                "Decision Answer",
                f"{description}\nAnswer: {selected}",
                "=" * 78,
            )
            if submitted_justification:
                self._append_history(
                    "Justification",
                    submitted_justification,
                    "=" * 78,
                )
            return selected, submitted_justification

    def prompt_multiline(self, prompt: str, initial_text: str = "") -> str:
        instructions = (
            f"{prompt}\n\n"
            "Enter your content in the right panel. "
            "Use F4 to open the default editor ($VISUAL/$EDITOR)."
        )
        return self._prompt_text(
            title="Current Action Input",
            instructions=instructions,
            multiline=True,
            initial_text=initial_text,
        )

    def prompt_args_form(
        self,
        args: list[AutomatedActionArg],
        initial_values: dict[str, str] | None = None,
    ) -> dict[str, str]:
        if not args:
            return {}

        application_module = importlib.import_module("prompt_toolkit.application")
        key_binding_module = importlib.import_module("prompt_toolkit.key_binding")
        layout_module = importlib.import_module("prompt_toolkit.layout")
        dimension_module = importlib.import_module("prompt_toolkit.layout.dimension")
        widgets_module = importlib.import_module("prompt_toolkit.widgets")

        Application = getattr(application_module, "Application")
        KeyBindings = getattr(key_binding_module, "KeyBindings")
        HSplit = getattr(layout_module, "HSplit")
        Layout = getattr(layout_module, "Layout")
        VSplit = getattr(layout_module, "VSplit")
        Dimension = getattr(dimension_module, "Dimension")
        Frame = getattr(widgets_module, "Frame")
        TextArea = getattr(widgets_module, "TextArea")
        ScrollablePane = getattr(layout_module, "ScrollablePane", None)

        field_areas = []
        field_frames = []
        text_area_signature = inspect.signature(TextArea)
        supports_placeholder = "placeholder" in text_area_signature.parameters
        prefill = initial_values or {}
        for arg in args:
            initial_value = prefill.get(arg.name)
            if initial_value is None:
                initial_value = "" if arg.default is None else arg.default
            field_kwargs = {
                "text": initial_value,
                "multiline": False,
                "focusable": True,
                "scrollbar": False,
                "wrap_lines": False,
            }
            if supports_placeholder and arg.default is None:
                field_kwargs["placeholder"] = f"Enter value for {arg.name}"
            area = TextArea(**field_kwargs)
            field_areas.append(area)
            title = (
                f"{arg.name}"
                if arg.default is None
                else f"{arg.name} (default: {arg.default})"
            )
            field_frames.append(
                Frame(area, title=title, height=Dimension(min=3, max=3))
            )

        instructions_area = TextArea(
            text=(
                "Edit all automated action arguments in this screen.\n"
                "Values are taken exactly as entered.\n"
                "Cleared fields become empty-string values."
            ),
            read_only=True,
            focusable=True,
            scrollbar=True,
            wrap_lines=True,
        )
        left_area = TextArea(
            text=self._history_text(),
            read_only=True,
            focusable=True,
            scrollbar=True,
            wrap_lines=False,
        )
        status_area = TextArea(
            text="", read_only=True, focusable=False, wrap_lines=True
        )
        key_bindings = KeyBindings()

        form_body = HSplit(field_frames)
        form_container = (
            ScrollablePane(form_body) if ScrollablePane is not None else form_body
        )

        def _focused_field_index() -> int:
            for index, field in enumerate(field_areas):
                if layout.has_focus(field):
                    return index
            return -1

        def _status_text() -> str:
            focused_index = _focused_field_index()
            if focused_index >= 0:
                active_name = args[focused_index].name
                return (
                    "Tab/Shift-Tab: switch focus | Ctrl-S/F2: submit | "
                    f"Arg {focused_index + 1}/{len(args)}: {active_name}"
                )
            if layout.has_focus(left_area):
                return "Tab/Shift-Tab: switch focus | Up/Down/PgUp/PgDn: scroll history"
            return "Tab/Shift-Tab: switch focus | Ctrl-S/F2: submit"

        @key_bindings.add("tab")
        def _focus_next(event) -> None:  # type: ignore[no-untyped-def]
            if event.app.layout.has_focus(left_area):
                event.app.layout.focus(field_areas[0])
            else:
                focused_index = _focused_field_index()
                if focused_index == -1 or focused_index >= len(field_areas) - 1:
                    event.app.layout.focus(left_area)
                else:
                    event.app.layout.focus(field_areas[focused_index + 1])
            status_area.text = _status_text()

        @key_bindings.add("s-tab")
        def _focus_previous(event) -> None:  # type: ignore[no-untyped-def]
            if event.app.layout.has_focus(left_area):
                event.app.layout.focus(field_areas[-1])
            else:
                focused_index = _focused_field_index()
                if focused_index <= 0:
                    event.app.layout.focus(left_area)
                else:
                    event.app.layout.focus(field_areas[focused_index - 1])
            status_area.text = _status_text()

        @key_bindings.add("c-s")
        @key_bindings.add("f2")
        def _submit(event) -> None:  # type: ignore[no-untyped-def]
            values = {arg.name: field.text for arg, field in zip(args, field_areas)}
            event.app.exit(result=values)

        @key_bindings.add("c-c")
        def _abort(_event) -> None:  # type: ignore[no-untyped-def]
            raise KeyboardInterrupt

        layout = Layout(
            HSplit(
                [
                    VSplit(
                        [
                            Frame(left_area, title="Analysis Progress"),
                            HSplit(
                                [
                                    Frame(
                                        instructions_area,
                                        title="Argument Input",
                                        height=Dimension(min=4, max=6),
                                    ),
                                    Frame(
                                        form_container,
                                        title="Automated Action Arguments",
                                        height=Dimension(weight=1),
                                    ),
                                ],
                                height=Dimension(weight=1),
                            ),
                        ],
                        height=Dimension(weight=1),
                    ),
                    Frame(
                        status_area,
                        title="Keys",
                        height=Dimension(min=3, max=3),
                    ),
                ],
                height=Dimension(weight=1),
            ),
            focused_element=field_areas[0],
        )

        app = Application(layout=layout, key_bindings=key_bindings, full_screen=True)
        pre_run_callables = getattr(app, "pre_run_callables", None)
        if isinstance(pre_run_callables, list):
            pre_run_callables.append(lambda: self._set_left_panel_scroll(left_area))
            pre_run_callables.append(
                lambda: setattr(status_area, "text", _status_text())
            )
        else:
            self._set_left_panel_scroll(left_area)
            status_area.text = _status_text()
        submitted = app.run()
        self._store_left_panel_scroll_state(left_area)
        if submitted is None:
            raise KeyboardInterrupt
        if not isinstance(submitted, dict):
            raise RuntimeError("Argument form submission returned invalid payload")
        return {str(name): str(value) for name, value in submitted.items()}

    def run_command(self, command: str) -> int:
        application_module = importlib.import_module("prompt_toolkit.application")
        key_binding_module = importlib.import_module("prompt_toolkit.key_binding")
        layout_module = importlib.import_module("prompt_toolkit.layout")
        dimension_module = importlib.import_module("prompt_toolkit.layout.dimension")
        widgets_module = importlib.import_module("prompt_toolkit.widgets")

        Application = getattr(application_module, "Application")
        KeyBindings = getattr(key_binding_module, "KeyBindings")
        HSplit = getattr(layout_module, "HSplit")
        Layout = getattr(layout_module, "Layout")
        Dimension = getattr(dimension_module, "Dimension")
        Frame = getattr(widgets_module, "Frame")
        TextArea = getattr(widgets_module, "TextArea")

        process = subprocess.Popen(
            command,
            shell=True,
            cwd=_workspace_root(),
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
        )

        left_area = TextArea(
            text=self._history_text(),
            read_only=True,
            focusable=True,
            scrollbar=True,
            wrap_lines=False,
        )
        status_area = TextArea(
            text="Running command... Ctrl-C to interrupt",
            read_only=True,
            focusable=False,
            wrap_lines=True,
        )
        key_bindings = KeyBindings()

        @key_bindings.add("c-c")
        def _abort(_event) -> None:  # type: ignore[no-untyped-def]
            if process.poll() is None:
                process.terminate()
            raise KeyboardInterrupt

        layout = Layout(
            HSplit(
                [
                    Frame(left_area, title="Analysis Progress"),
                    Frame(status_area, title="Command", height=Dimension(min=3, max=3)),
                ],
                height=Dimension(weight=1),
            ),
            focused_element=left_area,
        )
        app = Application(layout=layout, key_bindings=key_bindings, full_screen=True)

        def _stream_output() -> None:
            self._append_history("Command Output", "", "-" * 78)
            self._refresh_left_panel(left_area, app)
            first_output = True
            if process.stdout is not None:
                for line in process.stdout:
                    self._append_to_latest_history(
                        line,
                        separate_from_header=first_output,
                    )
                    first_output = False
                    self._refresh_left_panel(left_area, app)
            return_code = process.wait()
            self._refresh_left_panel(left_area, app)
            app.exit(result=return_code)

        worker = threading.Thread(target=_stream_output, daemon=True)

        pre_run_callables = getattr(app, "pre_run_callables", None)
        if isinstance(pre_run_callables, list):
            pre_run_callables.append(lambda: self._set_left_panel_scroll(left_area))
            pre_run_callables.append(worker.start)
        else:
            self._set_left_panel_scroll(left_area)
            worker.start()

        return_code = app.run()
        worker.join(timeout=0.1)
        self._store_left_panel_scroll_state(left_area)
        if return_code is None:
            raise KeyboardInterrupt
        return int(return_code)
