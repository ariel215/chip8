
/// Represents an error encountered while parsing chip8 ASM
#[derive(Debug)]
pub struct ParseError{
    /// The putative mnemonic encountered
    pub mnemonic: String,
    /// Additional error message
    pub message: String
}

impl ParseError{
    pub fn new(mnemonic: &str, message: &str)-> Self{
        Self{
            mnemonic: mnemonic.to_string(),
            message: message.to_string()
        }
    }
}
