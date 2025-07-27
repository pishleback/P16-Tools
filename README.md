# P16 Overview

<img width="3011" height="2096" alt="2025-07-27_18 44 18" src="https://github.com/user-attachments/assets/af91c305-50c6-475f-ae44-ff7390219cea" />

# Running a program

Programs can be loaded by manually setting the 32x32 grid of levers on top of the CPU.

The P16 has 16 pages of program ROM, and the levers form page 0. The other 15 pages are comprised of 3 pages of torch ROM located directly below the levers, and 12 pages of barrel ROM located to the side. Schematics for populating these pages can be generated from an assembly file using the tools in this repository.

# Instruction Set and Assembly Language

## Operations



|     Nibble 1      | Nibble 2 | Nibble 3 | Nibble 4 | Nibble 5 |  ...  | Nibble n |
| :---------------: | :------: | :------: | :------: | :------: | :---: | :------: |
|       PASS        |   True   |  23.99   |          |          |       |          |
|      SQL Hat      |   True   |          |          |          |       |          |
|  Codecademy Tee   |  False   |  19.99   |          |          |       |          |
| Codecademy Hoodie |  False   |  42.99   |          |          |       |          |




# Create a Schematic

`cargo run --manifest-path assembly/Cargo.toml -- -a prog.txt -o prog.json; py schematic/main.py prog.json prog.schem`

# Run a simulation

`cargo run --manifest-path assembly/Cargo.toml -q -- -a prog.txt -s`

