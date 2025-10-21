# v1.0
First tagged version.

# v2.0
Removed clear input queue button.
P16 now clears the input queue whenever it begins running.
P16 now clears the stack whenever it begins running.
Replaced KSETF (set flags according to the top of the stack) with DEL (pop from stack and don't write to a register or set the flags)
Replaced SETF (set the flags according to a register) with SETFLAGS (set the flags according to the nibble in the register argument)