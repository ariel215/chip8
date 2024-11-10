Chip8Emu
--------

An emulator for the classic virtual architechture
### Usage

`chip8 <ROM> [-s/--speed SPEED] [-d/--debug]`

### Keyboard

(Chip8 key is listed, QWERTY key is in parentheses)

---------------------------------
| 1 (1) | 2 (2) | 3 (3) | C (4) |
---------------------------------
| 4 (Q) | 5 (W) | 6 (E) | D (R) |
---------------------------------
| 7 (A) | 8 (S) | 9 (D) | E (F) |
----------------------------------
| A (Z) | 0 (X) | B (C) | F (V) |
---------------------------------

To better visualize the keyboard, imagine a phone keyboard
starting at 1, with A and B next to 0, and then the last four 
digits down the side.

#### Other keys:
- Press `[spacebar]` or `p` to pause/unpause
- Press `.` to toggle debug mode
- Press `[enter]` to step through the program

#### Debug mode:

Debug mode splits the screen into 4:
- Chip8 display in top left
- Assembly instructions in bottom left
- Memory contents in top right
- register contents in bottom right

While in debug mode, you can scroll through the program instructions, and click to the left of any instruction to set a breakpoint, represented by a red circle. Clicking a second time
will remove the breakpoint. The program will automatically pause when it hits a breakpoint.

### Resources

- [https://en.wikipedia.org/wiki/CHIP-8]
- [http://devernay.free.fr/hacks/chip8/C8TECH10.HTM]

