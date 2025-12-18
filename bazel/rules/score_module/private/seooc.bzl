"""
Backwards compatibility wrapper for safety_element_out_of_context macro.

This file re-exports the macro from its new location in score_seooc.bzl.
It exists for backwards compatibility and can be removed once all references
are updated to use score_seooc.bzl directly.
"""

load("//bazel/rules/score_module/private:score_seooc.bzl", _safety_element_out_of_context = "safety_element_out_of_context")

# Re-export the macro for backwards compatibility
safety_element_out_of_context = _safety_element_out_of_context
