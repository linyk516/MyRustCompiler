Typecheck examples
==================

Naming convention:

- `ok_*.txt`: should compile with `Diagnostics 0`.
- `err_*.txt`: should compile far enough to typecheck and report at least one diagnostic.

These files cover the currently implemented typecheck surface:

- function signatures and calls
- explicit return and tail expression returns
- let annotations and local inference
- block, if, loop, while, for-range
- arrays, tuples, field access and indexing
- references, mutable references, dereference and assignment
- type errors that are expressible in the current source language

Some internal inference errors, such as occurs-check failures, are covered by unit tests rather
than source examples because the current language syntax cannot construct such recursive types.
