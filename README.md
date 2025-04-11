# Assembly Interpreter

[![Tests](https://github.com/apavazza/asm-interpreter/actions/workflows/tests.yml/badge.svg)](https://github.com/apavazza/asm-interpreter/actions/workflows/tests.yml)
[![Build and Create Release](https://github.com/apavazza/asm-interpreter/actions/workflows/build-and-release.yml/badge.svg)](https://github.com/apavazza/asm-interpreter/actions/workflows/build-and-release.yml)
![Rust version](https://img.shields.io/badge/Rust-1.85.1-brightgreen.svg)

Assembly Interpreter is a simple interactive command-line tool built in Rust. It simulates basic assembly language operations for educational and testing purposes.

## Supported Instructions

- **MOV `<register>, <value>`**  
  Sets the given register to a specified value. The value can be an immediate constant prefixed with `#` (supports hexadecimal with `#0x` and decimal, e.g. `#15`) or the value from another valid register.  
  *Example*: `MOV r0, #15`

- **ADD `<dest_register>, <reg_operand>, <operand>`**  
  Adds the value in the first operand register and the second operand (which may be an immediate constant or a register) and stores the result in the destination register.  
  *Example*: `ADD r0, r1, #5`

- **SUB `<dest_register>, <reg_operand>, <operand>`**  
  Subtracts the second operand (immediate or register value) from the value in the first operand register and stores the result in the destination register.  
  *Example*: `SUB r0, r1, r2`

- **LSL `<dest_register>, <source_register>, <shift_amount>`**  
  Performs a logical left shift on the source register by the specified shift amount and stores the result in the destination register.  
  *Example*: `LSL r0, r1, #2`

- **LSR `<dest_register>, <source_register>, <shift_amount>`**  
  Performs a logical right shift on the source register by the specified shift amount and stores the result in the destination register.  
  *Example*: `LSR r0, r1, #3`

- **ASR `<dest_register>, <source_register>, <shift_amount>`**  
  Performs an arithmetic right shift on the source register by the specified amount and stores the result in the destination register.  
  *Example*: `ASR r0, r1, #1`

- **ROR `<dest_register>, <source_register>, <rotate_amount>`**  
  Rotates the bits of the source register to the right by the specified rotate amount and stores the result in the destination register.  
  *Example*: `ROR r0, r1, #4`

- **RRX `<dest_register>, <source_register>`**  
  Performs a rotate-right with extend (RRX) on the source register (rotates right by 1 bit using an assumed zero carry) and stores the result in the destination register.  
  *Example*: `RRX r0, r1`

- **PRINT `<register>`**  
  Displays the current value of the specified register.  
  *Example*: `PRINT r0`

- **EXIT**  
  Terminates the program.

## Example Usage

```shell
MOV r1, #5
MOV r2, #10
ADD r3, r1, r2
SUB r4, r2, r1
LSL r5, r1, #3
LSR r6, r2, #1
ASR r7, r1, #2
ROR r8, r2, #2
RRX r9, r1
PRINT r3
PRINT r4
EXIT
```

## Additional Notes

- Registers that have not been explicitly set are assumed to have a default value of `0`.
- The interpreter expects commands to be well-formed and does not perform extensive input validation.
- Commas are required between command arguments as shown in the examples above.

## License

This software is provided under the terms of the [GNU General Public License v3.0](LICENSE).