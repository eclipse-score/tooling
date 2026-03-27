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

import unittest
from types import SimpleNamespace
from unittest import mock

from manual_analysis.interactive_runner_ui_split import _SplitPaneUI
from manual_analysis.yaml_schema import AutomatedActionArg


class InteractiveRunnerUiTest(unittest.TestCase):

    def test_history_text_and_separators(self) -> None:
        ui = _SplitPaneUI()
        ui.print_header("Manual Action")
        ui.show_text("Recorded manual action result:", "Collected notes")
        history_text = ui._history_text()
        self.assertIn("#" * 78, history_text)
        self.assertIn("=" * 78, history_text)
        self.assertIn("Manual Action", history_text)
        self.assertIn("Collected notes", history_text)

    def test_left_panel_preserves_scroll_when_user_scrolled_up(self) -> None:
        ui = _SplitPaneUI()

        scrolled_up_area = SimpleNamespace(
            text="line 1\nline 2\nline 3\nline 4\nline 5",
            buffer=SimpleNamespace(cursor_position=0),
            window=SimpleNamespace(
                vertical_scroll=1,
                render_info=SimpleNamespace(content_height=5, window_height=2),
            ),
        )
        ui._store_left_panel_scroll_state(scrolled_up_area)

        new_area = SimpleNamespace(
            text="line 1\nline 2\nline 3\nline 4\nline 5\nline 6",
            buffer=SimpleNamespace(cursor_position=0),
            window=SimpleNamespace(vertical_scroll=0),
        )
        ui._set_left_panel_scroll(new_area)

        self.assertFalse(ui._follow_left_panel_output)
        self.assertEqual(ui._left_panel_vertical_scroll, 1)
        self.assertEqual(new_area.window.vertical_scroll, 1)
        self.assertEqual(new_area.buffer.cursor_position, len("line 1\n"))

    def test_refresh_left_panel_follows_bottom_until_user_scrolls_up(self) -> None:
        ui = _SplitPaneUI()
        app = mock.Mock()
        area = SimpleNamespace(
            text="line 1\nline 2\nline 3\nline 4\nline 5",
            buffer=SimpleNamespace(cursor_position=0),
            window=SimpleNamespace(
                vertical_scroll=3,
                render_info=SimpleNamespace(content_height=5, window_height=2),
            ),
        )

        ui.show_text("Section", "line 1\nline 2\nline 3\nline 4\nline 5")
        ui._refresh_left_panel(area, app)
        self.assertTrue(ui._follow_left_panel_output)
        app.invalidate.assert_called_once()

        app.reset_mock()
        area.window.vertical_scroll = 1
        area.window.render_info = SimpleNamespace(
            content_height=area.text.count("\n") + 1,
            window_height=2,
        )
        ui._append_to_latest_history("\nline 6")
        ui._refresh_left_panel(area, app)

        self.assertFalse(ui._follow_left_panel_output)
        self.assertEqual(ui._left_panel_vertical_scroll, 1)
        self.assertEqual(area.window.vertical_scroll, 1)
        app.invalidate.assert_called_once()

    def test_split_pane_prompt_choice_records_answer_in_history(self) -> None:
        ui = _SplitPaneUI()

        with mock.patch.object(ui, "_prompt_text", return_value="Yes") as prompt_text:
            answer = ui.prompt_choice("Proceed?", ["Yes", "No"], default_option="No")

        self.assertEqual(answer, "Yes")
        prompt_text.assert_called_once()
        history = ui._history_text()
        self.assertIn("Decision Answer", history)
        self.assertIn("Proceed?", history)
        self.assertIn("Answer: Yes", history)

    def test_split_pane_prompt_multiline_delegates_to_prompt_text(self) -> None:
        ui = _SplitPaneUI()

        with mock.patch.object(
            ui, "_prompt_text", return_value="line 1\nline 2"
        ) as prompt_text:
            value = ui.prompt_multiline("Describe outcome", initial_text="prefill")

        self.assertEqual(value, "line 1\nline 2")
        prompt_text.assert_called_once_with(
            title="Current Action Input",
            instructions=(
                "Describe outcome\n\n"
                "Enter your content in the right panel. "
                "Use F4 to open the default editor ($VISUAL/$EDITOR)."
            ),
            multiline=True,
            initial_text="prefill",
        )

    def test_split_pane_prompt_justification_delegates_to_prompt_text(self) -> None:
        ui = _SplitPaneUI()

        with mock.patch.object(
            ui, "_prompt_text", return_value="checked manually"
        ) as prompt_text:
            value = ui.prompt_justification(
                "Why this answer?",
                default_text="previous reason",
            )

        self.assertEqual(value, "checked manually")
        prompt_text.assert_called_once_with(
            title="Justification",
            instructions=(
                "Why this answer?\n\n"
                "Provide a short rationale for your selected answer. "
                "Leave empty if no justification is needed."
            ),
            multiline=True,
            initial_text="previous reason",
        )

    def test_split_pane_choice_with_justification_returns_both_values(self) -> None:
        ui = _SplitPaneUI()

        class FakeTextArea:
            def __init__(self, **kwargs) -> None:
                self.text = kwargs.get("text", "")
                self.read_only = kwargs.get("read_only", False)
                self.focusable = kwargs.get("focusable", True)
                self.scrollbar = kwargs.get("scrollbar", False)
                self.wrap_lines = kwargs.get("wrap_lines", True)
                self.multiline = kwargs.get("multiline", True)
                self.buffer = SimpleNamespace(cursor_position=0)
                self.window = SimpleNamespace(vertical_scroll=0, render_info=None)

        class FakeFrame:
            def __init__(self, body, title=None, height=None) -> None:
                self.body = body
                self.title = title
                self.height = height

        class FakeVSplit:
            def __init__(self, children, height=None) -> None:
                self.children = children
                self.height = height

        class FakeHSplit:
            def __init__(self, children, height=None) -> None:
                self.children = children
                self.height = height

        class FakeDimension:
            def __init__(self, **kwargs) -> None:
                self.kwargs = kwargs

        class FakeLayout:
            def __init__(self, container, focused_element=None) -> None:
                self.container = container
                self.focused_element = focused_element
                self.current_control = focused_element

            def has_focus(self, element) -> bool:
                return self.current_control is element

            def focus(self, element) -> None:
                self.current_control = element

        class FakeKeyBindings:
            def __init__(self) -> None:
                self.handlers = {}

            def add(self, key):
                def decorator(func):
                    self.handlers[key] = func
                    return func

                return decorator

        class FakeApplication:
            def __init__(self, layout, key_bindings, full_screen) -> None:
                self.layout = layout
                self.key_bindings = key_bindings
                self.full_screen = full_screen
                self.pre_run_callables = []

            def run(self):
                for callback in self.pre_run_callables:
                    callback()
                return {"answer": "Yes", "justification": "verified in code"}

        fake_modules = {
            "prompt_toolkit.application": mock.Mock(Application=FakeApplication),
            "prompt_toolkit.key_binding": mock.Mock(KeyBindings=FakeKeyBindings),
            "prompt_toolkit.layout": mock.Mock(
                HSplit=FakeHSplit, Layout=FakeLayout, VSplit=FakeVSplit
            ),
            "prompt_toolkit.layout.dimension": mock.Mock(Dimension=FakeDimension),
            "prompt_toolkit.shortcuts": mock.Mock(message_dialog=mock.Mock()),
            "prompt_toolkit.widgets": mock.Mock(Frame=FakeFrame, TextArea=FakeTextArea),
        }

        with mock.patch(
            "manual_analysis.interactive_runner_ui_split.importlib.import_module",
            side_effect=lambda name: fake_modules[name],
        ):
            answer, justification = ui.prompt_choice_with_justification(
                "Proceed?",
                ["Yes", "No"],
                default_option="No",
            )

        self.assertEqual(answer, "Yes")
        self.assertEqual(justification, "verified in code")

    def test_split_pane_prompt_args_form_returns_submitted_values(self) -> None:
        ui = _SplitPaneUI()

        class FakeTextArea:
            def __init__(self, **kwargs) -> None:
                self.text = kwargs.get("text", "")
                self.read_only = kwargs.get("read_only", False)
                self.focusable = kwargs.get("focusable", True)
                self.scrollbar = kwargs.get("scrollbar", False)
                self.wrap_lines = kwargs.get("wrap_lines", True)
                self.multiline = kwargs.get("multiline", True)
                self.placeholder = kwargs.get("placeholder")
                self.buffer = SimpleNamespace(cursor_position=0)
                self.window = SimpleNamespace(vertical_scroll=0, render_info=None)

        class FakeFrame:
            def __init__(self, body, title=None, height=None) -> None:
                self.body = body
                self.title = title
                self.height = height

        class FakeVSplit:
            def __init__(self, children, height=None) -> None:
                self.children = children
                self.height = height

        class FakeHSplit:
            def __init__(self, children, height=None) -> None:
                self.children = children
                self.height = height

        class FakeDimension:
            def __init__(self, **kwargs) -> None:
                self.kwargs = kwargs

        class FakeLayout:
            def __init__(self, container, focused_element=None) -> None:
                self.container = container
                self.focused_element = focused_element
                self.current_control = focused_element

            def has_focus(self, element) -> bool:
                return self.current_control is element

        class FakeKeyBindings:
            def __init__(self) -> None:
                self.handlers = {}

            def add(self, key):
                def decorator(func):
                    self.handlers[key] = func
                    return func

                return decorator

        class FakeApplication:
            def __init__(self, layout, key_bindings, full_screen) -> None:
                self.layout = layout
                self.key_bindings = key_bindings
                self.full_screen = full_screen
                self.pre_run_callables = []

            def run(self):
                for callback in self.pre_run_callables:
                    callback()
                return {"subject": "workspace", "mode": "fast"}

        fake_modules = {
            "prompt_toolkit.application": mock.Mock(Application=FakeApplication),
            "prompt_toolkit.key_binding": mock.Mock(KeyBindings=FakeKeyBindings),
            "prompt_toolkit.layout": mock.Mock(
                HSplit=FakeHSplit, Layout=FakeLayout, VSplit=FakeVSplit
            ),
            "prompt_toolkit.layout.dimension": mock.Mock(Dimension=FakeDimension),
            "prompt_toolkit.widgets": mock.Mock(Frame=FakeFrame, TextArea=FakeTextArea),
        }

        with mock.patch(
            "manual_analysis.interactive_runner_ui_split.importlib.import_module",
            side_effect=lambda name: fake_modules[name],
        ):
            values = ui.prompt_args_form(
                [
                    AutomatedActionArg(name="subject", default=""),
                    AutomatedActionArg(name="mode", default="default"),
                ],
                initial_values={"subject": "prefilled"},
            )

        self.assertEqual(values, {"subject": "workspace", "mode": "fast"})


if __name__ == "__main__":
    unittest.main()
