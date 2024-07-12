use std::num::ParseIntError;

pub struct ParseError;

impl From<ParseIntError> for ParseError{
    fn from(_value: ParseIntError) -> Self {
        Self{}
    }
}

impl From<()> for ParseError{
    fn from(_value: ()) -> Self {
        Self{}
    }
}