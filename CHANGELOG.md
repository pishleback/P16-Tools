# WIP
Instruction changes:
 - Added `.BREAK` command for breakpoints.

# v2.0
Hardware changes:
 - Removed clear input queue button.
 - P16 now clears the input queue whenever it begins running.
 - P16 now clears the stack whenever it begins running.
 - Added logic to reboot cleanly if turned off and on again quickly while running.
 - Added wiring to support new `SETFLAGS` instruction

Instruction changes:
 - Replaced `KSETF` (set flags according to the top of the stack) with `DEL` (pop from stack and don't write to a register or set the flags).
 - Replaced `SETF` (set the flags according to a register) with `SETFLAGS` (set the flags according to the nibble in the register argument).
 - Rename `PSETF` to `POPNOOP`.

# v1.0
First tagged version.