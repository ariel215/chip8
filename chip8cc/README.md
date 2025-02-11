Chip8CC
-------

### A C-like compiler for the Chip8






## Roadmap
1. Labels (completed)
1. static data:
    - BYTES pseudoinstruction
    - followed by 1 or more bytes in hex
    - terminated by a semicolon
    - implement layout
3. ISel
   - switch from chip8 ASM to low-level register IR
   - add pointers and pointer dereferencing
   - add arrays and structs
2. RegAlloc
   - expand registers from v0..v15 to an arbitrary number
5. Functions, stack frames, stack pointer
6. Types and type-checking
7. TBD...