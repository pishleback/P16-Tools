# WIP

Hardware changes:
 - Fixed missing dust for writing to the input queue after a RAM read instruction.
 - Fixed bug where the RAM hex to bin decoders were async causing occational bit flips when loading a RAM page.
 - Fixed an addressing bug with the program cache where addresses 110xxxxx would all read blank leading to a series of output instructions being executed in that section of the program.
 - Fixed timing bug where the top four layers of program cache (the second half of the cache) would all be off by one address when reading a page from RAM.
 - Fixed hardware bug where instructions B2r (write without pop) and B3r (write with pop) were swapped.

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
 - Added `.BREAK` command for breakpoints.

# v1.0
First tagged version.