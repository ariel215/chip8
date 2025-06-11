Chip8CC
-------

### A C-like compiler for the Chip8


## Roadmap
1. Labels (completed)
1. static data (completed)
    - BYTES pseudoinstruction
    - followed by 1 or more bytes in hex
    - terminated by a semicolon
2. basic blocks / cfg 
3. ISel
   - switch from chip8 ASM to low-level register IR
   - add pointers and pointer dereferencing
   - add arrays and structs
2. RegAlloc
   - expand registers from v0..v15 to an arbitrary number
5. Functions, stack frames, stack pointer
6. Types and type-checking
7. TBD...


## Thoughts

### How to do stores/loads in chip8

The Chip8 instruction set doesn't let you load memory into arbitrary registers,
but only the first N, which means that to load X into vN
- push v0
- load X into v0
- mov vN <- v0
- pop v0

... it's gross but it works
