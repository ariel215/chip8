Chip8CC
-------

### A C-like compiler for the Chip8






## Roadmap
1. Labels (completed)
1a. static data:
    - BYTES pseudoinstruction
    - followed by 1 or more bytes in hex
    - terminated by a semicolon
2. RegAlloc
   - expand registers from v0..v15 to an arbitrary number
3. ISel
   - replace asm instructions with more standard operations
   - add pointers and pointer dereferencing
4. Arrays and structs
5. Functions, stack frames, stack pointer
6. Types and type-checking
7. TBD...