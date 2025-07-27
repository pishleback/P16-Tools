# P16 Overview

<img width="3011" height="2096" alt="2025-07-27_18 44 18" src="https://github.com/user-attachments/assets/af91c305-50c6-475f-ae44-ff7390219cea" />

# Running a program

Programs can be loaded by manually setting the 32x32 grid of levers on top of the CPU.

The P16 has 16 pages of program ROM, and the levers form page 0. The other 15 pages are comprised of 3 pages of torch ROM located directly below the levers, and 12 pages of barrel ROM located to the side. Schematics for populating these pages can be generated from an assembly file using the tools in this repository.

# Instruction Set and Assembly Language

## Operations



|    Assembly     |                                                    Description                                                     |                            Nibbles                             |
| :-------------: | :----------------------------------------------------------------------------------------------------------------: | :------------------------------------------------------------: |
| `..ROM <page>`  | All instructions which follow are placed in ROM page #`page` until told otherwise where `page` is between 0 and 15 |                                                                |
|     `..RAM`     |                        All instructions which follow are placed in RAM until told otherwise                        |                                                                |
|     `PASS`      |                                                    Does nothing                                                    |                              `0`                               |
| `VALUE <value>` |                                            Push `value` onto the stack                                             |  `1VVVV` where `VVVV` is the 16-bit representation of `value`  |
| `JUMP <label>`  |                                       Continue execution from label `label`                                        | `2AA` where `AA` is the address of `label` in the current page |




# Create a Schematic

`cargo run --manifest-path assembly/Cargo.toml -- -a prog.txt -o prog.json; py schematic/main.py prog.json prog.schem`

# Run a simulation

`cargo run --manifest-path assembly/Cargo.toml -q -- -a prog.txt -s`

