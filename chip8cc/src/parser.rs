use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric0, digit1, hex_digit1, multispace0},
    combinator::{map_res, not, opt, peek, recognize, verify},
    error::Error,
    multi::many0,
    number::complete::hex_u32,
    sequence::{delimited, terminated},
    *,
};
use nom_supreme::{error::ErrorTree, final_parser::final_parser, ParserExt};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    Assign,
    Plus,
    Minus,
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

type PResult<I, O> = nom::IResult<I, O, ErrorTree<I>>;

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

pub fn parse_expression(input: &str) -> PResult<&str, ParseNode> {
    let terminal1 = alt((parse_number, parse_variable)).terminated(multispace0);
    let terminal2 = alt((parse_number, parse_variable))
        .terminated(multispace0)
        .context("needed expression here");

    let operator = alt((tag("+"), tag("-"))).map(|op| {
        if op == "+" {
            Operator::Plus
        } else {
            Operator::Minus
        }
    });
    (
        terminal1,
        multi::many0((multispace0, operator, multispace0, terminal2)),
    )
        .map(
            |(left, right): (ParseNode, Vec<(&str, Operator, &str, ParseNode)>)| {
                right.into_iter().fold(left, |acc, (_, op, _, right)| {
                    ParseNode::Binary(BinaryOp {
                        operator: op,
                        left: Box::new(acc),
                        right: Box::new(right),
                    })
                })
            },
        )
        .parse(input)
}

pub fn parse_assign(input: &str) -> PResult<&str, ParseNode> {
    let mut parser = (
        parse_variable,
        multispace0,
        tag("="),
        multispace0,
        parse_expression,
    )
        .context("assignment");
    let (rest, (var, _, _, _, val)) = Parser::parse(&mut parser, input)?;
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
    use super::*;

    macro_rules! passes {
        ($parser: ident, $input: expr) => {
            let result = $parser($input);
            assert!(result.is_ok(), "failed to parse {:?}: {:?}", $input, result);
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
            let input = format!("0x{:x} ", i);
            passes!(parse_unsigned, &input);
        }
        for i in u8::MIN..=u8::MAX {
            let input = format!("{} ", i);
            passes!(parse_unsigned, &input);
        }

        let input = "100 ";
        assert!(parse_unsigned(input).is_ok());
        let input = "0xc3 ";
        assert!(parse_unsigned(input).is_ok());
    }

    #[test]
    fn test_unsigned_failures() {
        fails!(parse_unsigned, "-1");
        fails!(parse_unsigned, "0xfff");
    }

    #[test]
    fn test_parse_number() {
        let input = "100 ";
        assert!(parse_number(input).is_ok());
        let input = "0xc3 ";
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
        passes!(parse_assign, "x = 1 ;\n");
        passes!(parse_assign, "x = y");
    }

    #[test]
    fn test_parse_add() {
        passes!(parse_expression, "1 + 2");
        passes!(parse_expression, "x + y + 1");
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
    fn test_parse_statements() {
        passes!(parse_statements, "x=1;");
        passes!(parse_statements, "x=1;\ny=3;\nz=y;");
        fails!(parse_statements, "x = 1 y = 3 z = y");
    }
}
