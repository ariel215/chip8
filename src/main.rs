use std::{thread::sleep, time::Duration};

use chip8::{Emulator, Instruction};


fn main() {
    let mut emulator = Emulator::windowed();
    let instructions: Vec<u8> = [
        Instruction::SetMemPtr(0),
        Instruction::SetImm(0, 2),
        Instruction::SetImm(1,2),
        Instruction::Draw(0, 1, 5),
        Instruction::SetMemPtr(5),
        Instruction::SetImm(1, 15),
        Instruction::Draw(0, 1, 5),
        Instruction::Jump(0x200 + 14)
    ].into_iter().map(|i| <Instruction as Into<u16>>::into(i).to_be_bytes())
    .flatten().collect();
    emulator.load_rom(&instructions);
    emulator.run()
}
