use chip8::Instruction;


fn main() {
    let mut _memory = chip8::Memory::default();
    let mut _registers = chip8::Registers::default();
    let instructions: Vec<u8> = [
        Instruction::SetMemPtr(0),
        Instruction::SetImm(0, 0),
        Instruction::SetImm(1,0),
        Instruction::Draw(0, 1, 5),
        Instruction::SetMemPtr(5),
        Instruction::SetImm(1, 15),
        Instruction::Draw(0, 1, 5)
    ].into_iter().map(|i| <Instruction as Into<u16>>::into(i).to_be_bytes())
    .flatten().collect();
    _memory.load_rom(&instructions);
    for _ in 0..instructions.len(){
        chip8::do_instruction(&mut _memory, &mut _registers)
    }
    let contents = _memory.print_display();
    print!("{}",&contents);
}
