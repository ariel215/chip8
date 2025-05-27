use nom::{bytes::tag, character::{complete::hex_digit1, digit1}, combinator::{map_res, not, peek, verify}, error::Error, number::complete::hex_u32, *};

pub enum ParseNode{
    Unsigned(u8), 
    Signed(i8)
}

pub fn parse_unsigned(input: &str) -> nom::IResult<&str, u8> {
    if let Ok((remaining, _)) = branch::alt((tag::<&str, &str, Error<&str>>("0x"), tag("0X"))).parse(input){
        map_res(hex_digit1, 
        |v| u8::from_str_radix(v, 16)
        ).parse(remaining)
    } else {
        map_res(digit1(), |v| u8::from_str_radix(v, 10)).parse(input)
    }
}


pub fn parse_number(input: &str) -> IResult<&str, ParseNode>{
        if let Ok((remaining, _)) = tag::<&str, &str, Error<&str>>("-").parse(input){
            parse_unsigned(remaining).map(|(rest,v)| (rest, ParseNode::Signed(-(v as i8))))
        } else {
            parse_unsigned(input).map(|(rest,v)| (rest, ParseNode::Unsigned(v)))
        }
}


#[cfg(test)]
mod tests {
    use crate::parser::{parse_number, parse_unsigned};


    macro_rules! passes {
        ($parser: ident, $input: expr) => {
            let result = $parser($input);
            assert!(result.is_ok(), "failed to parse 0x{:?}: {:?}", $input, result)
        };
    }

    macro_rules! fails {
        ($parser: ident, $input: expr) => {
            let result = $parser($input);
            assert!(result.is_err(), "{:?} parsed as {:?}", $input, result)
        };
    }

    #[test]
    fn test_unsigned(){
        for i in u8::MIN..=u8::MAX{
            let input = format!("0x{:x} ", i);
            passes!(parse_unsigned, &input);
        }
        for i in u8::MIN..=u8::MAX{
            let input = format!("{} ", i);
            passes!(parse_unsigned, &input);
        }

        let input = "100 ";
        assert!(parse_unsigned(input).is_ok());
        let input = "0xc3 ";
        assert!(parse_unsigned(input).is_ok());
    }

    #[test]
    fn test_unsigned_failures(){
        fails!(parse_unsigned, "-1");
        fails!(parse_unsigned, "0xfff");
    }

    #[test]
    fn test_parse_number(){
        let input = "100 ";
        assert!(parse_number(input).is_ok());
        let input = "0xc3 ";
        assert!(parse_number(input).is_ok());
    }



}