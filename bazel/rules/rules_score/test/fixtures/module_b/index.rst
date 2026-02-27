Module B Documentation
======================

This is the documentation for Module B.

.. document:: Documentation for Module B
   :id: doc__module_fixtures_module_b
   :status: valid
   :safety: ASIL_B
   :security: NO
   :realizes:

Overview
--------

Module B depends on both Module A and Module C.

Features
--------

.. needlist::
   :tags: module_b

Cross-Module References
-----------------------

This module references:

* :external+module_a_lib:doc:`index` from Module A
* :external+module_c_lib:doc:`index` from Module C
* Need reference to Module C :need:`doc__module_fixtures_module_c`
* Need reference to Module C :need:`doc__module_fixtures_module_d`

Dependencies
------------

Module B integrates functionality from both dependent modules.
