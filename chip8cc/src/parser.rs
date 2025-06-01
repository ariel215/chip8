use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric0, digit1, hex_digit1, multispace0},
    combinator::{map_res, not, opt, peek, recognize, verify},
    error::{Error, ParseError},
    multi::many0,
    number::complete::hex_u32,
    sequence::{delimited, terminated},
    *,
};
use nom_supreme::{error::ErrorTree, final_parser::final_parser, ParserExt};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    Times, 
    Divided,
    Plus,
    Minus,
    ShiftLeft,
    ShiftRight,
    LEq,
    Lt,
    Geq, 
    Gt,
    Eq,
    Neq,
    BitAnd,
    BitOr,
    BitXor,
    LogicAnd,
    LogicOr,
    Assign,

}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOp {
    pub operator: Operator,
    pub left: Box<ParseNode>,
    pub right: Box<ParseNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseNode {
    Unsigned(u8),
    Signed(i8),
    Var(String),
    Binary(BinaryOp),
}

type PResult<'a, I=&'a str, O=ParseNode> = nom::IResult<I, O, ErrorTree<I>>;

pub fn parse_unsigned(input: &str) -> PResult<&str, u8> {
    if let Ok((remaining, _)) =
        branch::alt((tag::<&str, &str, Error<&str>>("0x"), tag("0X"))).parse(input)
    {
        map_res(hex_digit1, |v| u8::from_str_radix(v, 16))
            .context("unsigned hex")
            .parse(remaining)
    } else {
        map_res(digit1, |v| u8::from_str_radix(v, 10))
            .context("unsigned decimal")
            .parse(input)
    }
}

pub fn parse_number(input: &str) -> PResult<&str, ParseNode> {
    if let Ok((remaining, _)) = tag::<&str, &str, Error<&str>>("-").parse(input) {
        parse_unsigned(remaining).map(|(rest, v)| (rest, ParseNode::Signed(-(v as i8))))
    } else {
        parse_unsigned(input).map(|(rest, v)| (rest, ParseNode::Unsigned(v)))
    }
}

pub fn parse_variable(input: &str) -> PResult<&str, ParseNode> {
    recognize(alpha1.and(alphanumeric0))
        .map(|value: &str| (ParseNode::Var(value.to_string())))
        .parse(input)
}

pub fn atom(input: &str) -> PResult {
    let parens = delimited(tag("("), parse_expression, tag(")"));
    alt((parse_number, parse_variable, parens)).parse(input)
}

pub fn left_associate<'a, T, O>(term: T, ops: O) -> impl FnMut(&'a  str) -> PResult
    where 
    O: ParserExt<&'a str, Operator, ErrorTree<&'a str>>,
    T: ParserExt<&'a str, ParseNode, ErrorTree<&'a str>> + Copy{
    let t1 = term.terminated(multispace0);
    let t2 = term.terminated(multispace0);
    let terms =        (t1, many0(
            (ops.terminated(multispace0), t2)));
    let mut parser = Parser::map(terms,
    |(left, rest)| rest.into_iter().fold(
                left, |acc, (op,right)| {
                    ParseNode::Binary(BinaryOp { operator: op, left: Box::new(acc), right: Box::new(right) })
                }
            )
     );
    move |input: &str| {
        parser.parse(input)
    }
}

macro_rules! operators {
    ($(($character: expr, $operator: expr)),*) => {
        alt(($(tag($character)),* ))
            .map(|c| match c {
                $($character => $operator,)*
                _ => unreachable!()
            })
    };
}

pub fn parse_product(input: &str) -> PResult<&str, ParseNode>{
    let operators = operators!(
        ("*", Operator::Times),
        ("/", Operator::Divided)
    );
    left_associate(atom, operators).parse(input)
}

pub fn parse_sum(input: &str) -> PResult<&str, ParseNode> {
        let ops = operators!(("+", Operator::Plus), ("-", Operator::Minus));
        left_associate(parse_product, ops)
        .parse(input)
}

pub fn parse_shift(input: &str) -> PResult {
    left_associate(parse_sum, operators!(
        ("<<", Operator::ShiftLeft),
        (">>", Operator::ShiftRight)
    )).parse(input)
}

pub fn parse_relation(input: &str) -> PResult {
    left_associate(parse_shift, operators!(
        ("<=", Operator::LEq),
        ("<", Operator::Lt),
        (">=", Operator::Geq),
        (">", Operator::Gt)
    )).parse(input)
}


pub fn parse_eq(input: &str) -> PResult {
    left_associate(parse_relation, operators!(
        ("==", Operator::Eq),
        ("!=", Operator::Neq)
    )).parse(input)
}

pub fn parse_and(input: &str) -> PResult {
    left_associate(parse_eq, 
        tag("&").map(|_| Operator::BitAnd)
    ).parse(input)
}

pub fn parse_or(input: &str) -> PResult {
    left_associate(parse_and, tag("|").map(|_|Operator::BitOr)
    ).parse(input)
}

pub fn parse_xor(input: &str) -> PResult {
    left_associate(parse_or, tag("^").map(|_|Operator::BitXor)
    ).parse(input)
}

pub fn parse_logical_and(input: &str) -> PResult {
    left_associate(parse_xor,     tag("&&").map(|_|Operator::LogicAnd)
    ).parse(input)
}

pub fn parse_logical_or(input: &str) -> PResult {
    left_associate(parse_logical_and,     tag("||").map(|_|Operator::LogicOr)
    ).parse(input)
}

pub fn parse_expression(input: &str) -> PResult {
    parse_logical_or(input)
}

pub fn parse_assign(input: &str) -> PResult<&str, ParseNode> {
    let mut parser = (
        parse_variable.terminated(multispace0),
        tag("=").terminated(multispace0),
        parse_logical_or,
    )
        .context("assignment");
    let (rest, (var,  _, val)) = Parser::parse(&mut parser, input)?;
    return Ok((
        rest,
        ParseNode::Binary(BinaryOp {
            operator: Operator::Assign,
            left: Box::new(var),
            right: Box::new(val),
        }),
    ));
}

/**
 * Parse a sequence of statements
 */
pub fn parse_statements(input: &str) -> Result<Vec<ParseNode>, ErrorTree<&str>> {
    let line = parse_assign.terminated(
        opt(multispace0)
            .and(tag(";"))
            .context("expected ';'")
            .and(opt(multispace0)),
    );
    final_parser(multi::many1(line))(input)
}

#[cfg(test)]
mod tests {
    use std::result;

    use super::*;

    macro_rules! passes {
        ($parser: ident, $input: expr) => {
            let result = $parser($input);
            assert!(result.is_ok(), "failed to parse {:?}: {:?}", $input, result);
            let (remainder, _) = result.unwrap();
            assert!(remainder.len() == 0, "failed to parse {} from {}", remainder, $input)
        };
    }

    macro_rules! fails {
        ($parser: ident, $input: expr) => {
            let result = $parser($input);
            assert!(result.is_err(), "{:?} parsed as {:?}", $input, result);
        };
    }

    macro_rules! u {
        ($val: expr) => {
            ParseNode::Unsigned($val)
        };
    }

    macro_rules! var {
        ($val: expr) => {
            ParseNode::Var($val.to_string())
        };
    }

    macro_rules! plus {
        ($l: expr, $r: expr) => {
            ParseNode::Binary(BinaryOp {
                operator: Operator::Plus,
                left: Box::new($l),
                right: Box::new($r),
            })
        };
    }
    macro_rules! times {
        ($l: expr, $r: expr) => {
            ParseNode::Binary(BinaryOp {
                operator: Operator::Times,
                left: Box::new($l),
                right: Box::new($r),
            })
        };
    }


    macro_rules! assign {
        ($l: expr, $r: expr) => {
            ParseNode::Binary(BinaryOp {
                operator: Operator::Assign,
                left: Box::new($l),
                right: Box::new($r),
            })
        };
    }

    #[test]
    fn test_unsigned() {
        for i in u8::MIN..=u8::MAX {
            let input = format!("0x{:x}", i);
            passes!(parse_unsigned, &input);
        }
        for i in u8::MIN..=u8::MAX {
            let input = format!("{}", i);
            passes!(parse_unsigned, &input);
        }
    }

    #[test]
    fn test_unsigned_failures() {
        fails!(parse_unsigned, "-1");
        fails!(parse_unsigned, "0xfff");
    }

    #[test]
    fn test_parse_number() {
        let input = "100";
        assert!(parse_number(input).is_ok());
        let input = "0xc3";
        assert!(parse_number(input).is_ok());
    }

    #[test]
    fn test_parse_variable() {
        passes!(parse_variable, "x");
        passes!(parse_variable, "x2");
        fails!(parse_variable, "2x");
    }

    #[test]
    fn test_parse_assign() {
        passes!(parse_assign, "x=1");
        passes!(parse_assign, "x = 1");
        // passes!(parse_assign, "x = 1 ;\n");
        passes!(parse_assign, "x = y");
    }

    #[test]
    fn test_parse_add() {
        passes!(parse_sum, "1 + 2");
        passes!(parse_sum, "x + y + 1");
    }

    #[test]
    fn test_parse_product(){
        passes!(parse_product, "2 * 3");
        passes!(parse_product, "x / 2");
    }


    #[test]
    fn test_atoms(){
        passes!(parse_sum, "1");
        passes!(parse_product, "2");
    }

    #[test]
    fn test_add_mul(){
        let input = "1 + 2 * 3 - 4";
        passes!(parse_sum,input);
        let (_,result) = parse_sum.complete().parse(input).unwrap();
        if let ParseNode::Binary(BinaryOp{operator, .. }) = &result {
            assert!(matches!(operator, Operator::Minus), "{:?}", result)
        } else { panic!("{:?}", result)}
    }


    #[test]
    fn test_parse_assign_add() {
        passes!(parse_assign, "x = 1 + 2");
        let (_, root) = parse_assign("x = 1 + 2").unwrap();
        if let ParseNode::Binary(BinaryOp {
            operator,
            left,
            right,
        }) = root
        {
            assert!(matches!(operator, Operator::Assign));
            assert!(matches!(*left, ParseNode::Var(_)));
            if let ParseNode::Binary(BinaryOp {
                operator,
                left,
                right,
            }) = *right
            {
                assert!(matches!(operator, Operator::Plus));
                assert!(matches!(*left, ParseNode::Unsigned(1)), "left: {:?}", *left);
                assert!(
                    matches!(*right, ParseNode::Unsigned(2)),
                    "right: {:?}",
                    *right
                );
            } else {
                panic!("bad sum: {:?}", *right)
            }
        } else {
            panic!("bad root")
        }
    }

    #[test]
    fn test_parse_add_multiple() {
        let input = "x = 1 + 2 + 3 + z";
        passes!(parse_assign, input);
        let (_, result) = parse_assign(input).unwrap();
        assert_eq!(
            result,
            assign!(
                var!("x"),
                plus!(plus!(plus!(u!(1), u!(2)), u!(3)), var!("z"))
            )
        );
    }

    #[test]
    fn test_parens(){
        let input = "(1 + 2) * (3 + 4)";
        passes!(parse_expression, input);
        let (_, result) = parse_expression(input).unwrap();
        assert_eq!(result,
            times!(plus!(u!(1),u!(2)), plus!(u!(3), u!(4)))
        )
    }

    #[test]
    fn test_parse_statements() {
        let inputs = [("x=1;",1), ("x=1;\ny=3;\nz=y;",3)];
        for (input, expected_len) in inputs.iter(){ 
            let result = parse_statements(&input);
            assert!(result.is_ok());
            assert!(result.unwrap().len() == *expected_len)
        }
        assert!(parse_statements("x = 1 y = 3 z = y").is_err());
    }
}
