Module A Documentation
======================

This is the documentation for Module A.

.. document:: Documentation for Module A
   :id: doc__module_fixtures_module_a
   :status: valid
   :safety: ASIL_B
   :security: NO
   :realizes: wp__component_arch

Overview
--------

Module A is a simple module that depends on Module C.

Features
--------

.. needlist::
   :tags: module_a

Cross-Module References
-----------------------

General reference to Module C :external+module_c_lib:doc:`index`.

Need reference to Module C :need:`doc__module_fixtures_module_c`.

Need reference to Module B :need:`doc__module_fixtures_module_b`.
