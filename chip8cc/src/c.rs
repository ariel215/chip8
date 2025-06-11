use bumpalo::{boxed::Box as BBox, Bump};
use pest::{
    iterators::{Pair, Pairs},
    pratt_parser::{self, Op, PrattParser},
    Parser as PestParser,
};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar/c.pest"]
pub struct CParser;

#[derive(Clone, Copy)]
struct ExprId(usize);

/**
 * AST Design
 *
 * Things to handle:
 * - name resolution
 *   - scopes
 * - typing
 */

// struct Function<'a>{
//     name: &'a str,
//     parameters: Vec<Variable>,
//     return_type: Type,
//     scope: ScopeId
// }

// struct Scope{
//     parent: Option<ScopeId>,
//     variables: Hash<Variable>,
//     statements: Vec<Statement>
// }
// struct Variable<'a>{
//     name: &'a str,
//     type_: Type,
// }

// /**
//  * Data Types
//  */
// enum NumericType{
//     U8,
//     U16,
//     S8,
//     S16
// }

// struct StructType<'a>{
//     name: Option<&'a str>,

// }

// struct PtrType{
//     n_stars: usize,
//     base_type: BasicType
// }

// enum Type{
//     Basic(BasicType),
//     Ptr(PtrType),

// }

struct ScopeId(usize);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
enum Value {
    Identifier { name: String },
    Constant { value: u64 },
    StringLiteral { value: String },
}

type BoxExpr<'a> = BBox<'a, Expression<'a>>;

#[derive(Debug)]
struct UnaryExpression<'a> {
    operand: BoxExpr<'a>,
    operation: Operation,
}

#[derive(Debug)]
struct BinaryExpression<'a> {
    left: BoxExpr<'a>,
    right: BoxExpr<'a>,
    operation: Operation,
}

#[derive(Debug)]
struct ExpressionList<'a> {
    expressions: Vec<BoxExpr<'a>>,
}

#[derive(Debug, Clone, Copy)]
enum Operation {
    // Unary operations
    Inc,        // ++
    Dec,        // --
    Sizeof,     //  sizeof
    Positive,   // +
    Negative,   //-
    AddrOf,     // &
    Deref,      // *
    Complement, //~
    Not,        // !,
    FieldOf,    // .
    Index,      // []
    // Binary operations
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Gt,
    Lt,
    Geq,
    Leq,
    BinaryAnd,
    BinaryOr,
    BinaryXor,
    LogicalAnd,
    LogicalOr,
    Call,
    Comma,
    Assign(AssignmentOp),
}

#[derive(Debug, Clone, Copy)]
enum AssignmentOp {
    Eq,
    MulEq,
    DivEq,
    ModEq,
    AddEq,
    SubEq,
    AndEq,
    OrEq,
}

#[derive(Debug)]
enum Expression<'a> {
    Primitive(Value),
    Unary(UnaryExpression<'a>),
    Binary(BinaryExpression<'a>),
}

macro_rules! left {
    ($expr: expr) => {
        match $expr {
            Expression::Binary(ref bin) => Some(&bin.left),
            _ => None,
        }
    };
}

macro_rules! right {
    ($expr: expr) => {
        match $expr {
            Expression::Binary(ref bin) => Some(&bin.right),
            _ => None,
        }
    };
}

macro_rules! operation {
    ($expr: expr) => {
        match $expr {
            Expression::Binary(ref bin) => Some(bin.operation),
            Expression::Unary(ref un) => Some(un.operation),
            Expression::Primitive(_) => None,
        }
    };
}

fn parse_constant<'a>(mut constant: Pairs<'a, Rule>) -> u64 {
    let inner = constant.next().unwrap();
    match inner.as_rule() {
        Rule::hex => {
            let suffix = inner
                .as_str()
                .strip_prefix("0x")
                .or(inner.as_str().strip_prefix("0X"))
                .unwrap();
            return u64::from_str_radix(suffix, 16).unwrap();
        }
        Rule::decimal => inner.as_str().parse::<u64>().unwrap(),
        _ => panic!(),
    }
}

fn parse_string<'a>(string: Pairs<'a, Rule>) -> String {
    todo!()
}

fn parse_expression<'input, 'arena>(
    arena: &'arena Bump,
    expression: Pairs<'input, Rule>,
) -> BoxExpr<'arena> {
    let pratt = PrattParser::new()
        .op(Op::infix(Rule::assign, pratt_parser::Assoc::Right))
        .op(Op::infix(Rule::add, pratt_parser::Assoc::Left))
        .op(Op::infix(Rule::mul, pratt_parser::Assoc::Left))
        .op(Op::prefix(Rule::prefix))
        .op(Op::postfix(Rule::postfix));

    pratt
        .map_primary(|primary| match primary.as_rule() {
            Rule::ident => BBox::new_in(
                Expression::Primitive(Value::Identifier {
                    name: primary.as_str().to_string(),
                }),
                arena,
            ),
            Rule::constant => BBox::new_in(
                Expression::Primitive(Value::Constant {
                    value: parse_constant(primary.into_inner()),
                }),
                arena,
            ),
            Rule::string_literal => BBox::new_in(
                Expression::Primitive(Value::StringLiteral {
                    value: parse_string(primary.into_inner()),
                }),
                arena,
            ),
            Rule::expression => parse_expression(arena, primary.into_inner()),
            _ => panic!("unexpected rule {:?}", primary.as_rule()),
        })
        .map_infix(|left, infix, right: BBox<'_, Expression<'_>>| {
            let operation = match infix.as_str() {
                "*" => Operation::Mul,
                "/" => Operation::Div,
                "%" => Operation::Mod,
                "+" => Operation::Add,
                "-" => Operation::Sub,
                "=" => Operation::Assign(AssignmentOp::Eq),
                "+=" => Operation::Assign(AssignmentOp::AddEq),
                "-=" => Operation::Assign(AssignmentOp::SubEq),
                "*=" => Operation::Assign(AssignmentOp::MulEq),
                "/=" => Operation::Assign(AssignmentOp::DivEq),
                "." => Operation::FieldOf,
                "->" => Operation::FieldOf,
                "[" => Operation::Index,
                "(" => Operation::Call,
                _ => panic!("unknown operator {:?}", infix.as_str()),
            };
            // Handle the case of function calls and array indexing

            BBox::new_in(
                Expression::Binary(BinaryExpression {
                    left,
                    right,
                    operation,
                }),
                arena,
            )
        })
        .map_prefix(|prefix, suffix| {
            let op = match prefix.clone().as_rule() {
                Rule::inc_op => Operation::Inc,
                Rule::dec_op => Operation::Dec,
                Rule::unary_op => match prefix.clone().as_str() {
                    "&" => Operation::AddrOf,
                    "*" => Operation::Deref,
                    "+" => Operation::Positive,
                    "-" => Operation::Negative,
                    "~" => Operation::Complement,
                    "!" => Operation::Not,
                    _ => panic!("unknown operator {:?}", prefix),
                },
                Rule::sizeof => Operation::Sizeof,
                _ => panic!("unknown rule {:?}", prefix),
            };
            BBox::new_in(
                Expression::Unary(UnaryExpression {
                    operation: op,
                    operand: suffix,
                }),
                arena,
            )
        })
        .map_postfix(|expression, postfix| match postfix.as_rule() {
            Rule::inc_op | Rule::dec_op => {
                let op = if postfix.as_str() == "++" {
                    Operation::Inc
                } else {
                    Operation::Dec
                };
                return BBox::new_in(
                    Expression::Unary(UnaryExpression {
                        operation: op,
                        operand: expression,
                    }),
                    arena,
                );
            }
            _ => panic!("unknown postfix operator {:?}", postfix),
        })
        .parse(expression)
}

#[cfg(test)]
mod tests {

    use super::*;

    macro_rules! test_parser {
        ($name: ident, $rule: expr, $input: literal) => {
            #[test]
            fn $name() {
                assert!(CParser::parse($rule, $input).is_ok())
            }
        };
    }

    test_parser!(test_add, Rule::expression, "1 + 2 - 3 + 4");
    test_parser!(test_add_mul, Rule::expression, "2 * 3 + 4 * 5");
    test_parser!(test_cast, Rule::expression, "(short) 3 * 4 + 5");
    test_parser!(test_ident, Rule::expression, "x + y");
    test_parser!(test_field, Rule::expression, "x.y");
    test_parser!(test_call, Rule::expression, "fn()");
    test_parser!(test_call_args, Rule::expression, "fn(1,2,x)");
    test_parser!(test_assign, Rule::expression, "x = y + 3");

    // test_parser!(test_decl_simple, Rule::decl, "int x");
    // test_parser!(test_decl_assign, Rule::decl, "int x = 3");
    // test_parser!(test_typedef, Rule::typedef_decl, "typedef struct mystruct mystruct_t" );

    macro_rules! test_pratt_parse {
        ($name: ident,  $input: literal) => {
            #[test]
            fn $name() {
                let pairs = CParser::parse(Rule::expression, $input).unwrap();
                let arena = Bump::new();
                parse_expression(&arena, pairs);
            }
        };
    }

    test_pratt_parse!(test_pratt_add, "1 + 2 + 3");

    #[test]
    fn test_precedence_1() {
        let arena = Bump::new();
        let input = "1 + 2 + 3";
        let pairs = CParser::parse(Rule::expression, input).unwrap();
        let root = parse_expression(&arena, pairs);
        if let Expression::Binary(exp) = &*root {
            assert!(matches!(*exp.left, Expression::Binary(_)));
        } else {
            panic!();
        }
    }

    #[test]
    fn test_precedence_add_mul() {
        let arena = Bump::new();
        let input = "1 * 2 + 3 * 4";
        let expression = CParser::parse(Rule::expression, input).unwrap();
        let root = parse_expression(&arena, expression);
        assert!(
            matches!(operation!(*root), Some(Operation::Add)),
            "{:?}",
            operation!(*root)
        );
        let left = left!(*root).unwrap();
        assert!(
            matches!(operation!(**left), Some(Operation::Mul)),
            "{:?} should be Mul",
            operation!(**left)
        );
        let right = right!(*root).unwrap();
        assert!(
            matches!(operation!(**right), Some(Operation::Mul)),
            "{:?} should be Mul",
            operation!(**right)
        );
    }

    #[test]
    fn test_expressions() {
        let arena = Bump::new();
        let input = "1 + 2 * 3 - 4 / 5";
        let expression = CParser::parse(Rule::expression, input).unwrap();
        let root = parse_expression(&arena, expression);
        assert!(matches!(operation!(*root), Some(Operation::Sub)));
        let left = left!(*root).unwrap();
        assert!(matches!(operation!(**left), Some(Operation::Add)));
        let right = right!(*root).unwrap();
        assert!(matches!(operation!(**right), Some(Operation::Div)));
    }

    #[test]
    fn test_ident_add() {
        let arena = Bump::new();
        let input = "x + y";
        let expression = CParser::parse(Rule::expression, input).unwrap();
        let root = parse_expression(&arena, expression);
        assert!(
            matches!(operation!(*root), Some(Operation::Add)),
            "{:?}",
            *root
        );
        let left = left!(*root).unwrap();
        assert!(
            matches!(**left, Expression::Primitive(Value::Identifier { name: _ })),
            "{:?}",
            **left
        );
        let right = right!(*root).unwrap();
        assert!(
            matches!(
                **right,
                Expression::Primitive(Value::Identifier { name: _ })
            ),
            "{:?}",
            **right
        );
    }

    #[test]
    fn test_assignments() {
        let arena = Bump::new();
        let input = "x = y + 3";
        let expression = CParser::parse(Rule::expression, input).unwrap();
        let root = parse_expression(&arena, expression);
        assert!(
            matches!(operation!(*root), Some(Operation::Assign(AssignmentOp::Eq))),
            "got {:?}",
            operation!(*root)
        );
        let left = left!(*root).unwrap();
        assert!(
            matches!(operation!(**left), None),
            "{:?}",
            operation!(**left)
        );
        let right = right!(*root).unwrap();
        assert!(
            matches!(operation!(**right), Some(Operation::Add)),
            "{:?}",
            operation!(**right)
        );
    }

    #[test]
    fn test_assignments_chained() {
        let arena = Bump::new();
        let input = "x = y = 3";
        let expression = CParser::parse(Rule::expression, input).unwrap();
        let root = parse_expression(&arena, expression);
        assert!(matches!(
            operation!(*root),
            Some(Operation::Assign(AssignmentOp::Eq))
        ));
        let left = left!(*root).unwrap();
        assert!(
            matches!(operation!(**left), None),
            "{:?}",
            operation!(**left)
        );
        let right = right!(*root).unwrap();
        assert!(
            matches!(
                operation!(**right),
                Some(Operation::Assign(AssignmentOp::Eq))
            ),
            "{:?}",
            operation!(**right)
        );
    }

    #[test]
    fn test_member_access() {
        let arena = Bump::new();
        let input = "x.y";
        let expression = CParser::parse(Rule::expression, input).unwrap();
        let root = parse_expression(&arena, expression);
        assert!(
            matches!(operation!(*root), Some(Operation::FieldOf)),
            "{:?}",
            operation!(*root)
        );
        let left = left!(*root).unwrap();
        assert!(
            matches!(**left, Expression::Primitive(Value::Identifier { name: _ })),
            "{:?}",
            **left
        );
    }

    #[test]
    fn test_ptr_access() {
        let arena = Bump::new();
        let input = "x->y";
        let expression = CParser::parse(Rule::expression, input).unwrap();
        let root = parse_expression(&arena, expression);
        assert!(
            matches!(operation!(*root), Some(Operation::FieldOf)),
            "{:?}",
            operation!(*root)
        );
        let left = left!(*root).unwrap();
        assert!(matches!(**left, Expression::Unary(_)), "{:?}", **left);
    }

    #[test]
    fn test_function_call() {
        let arena = Bump::new();
        let input = "fn(1,2,x)";
        let expression = CParser::parse(Rule::expression, input).unwrap();
        let root = parse_expression(&arena, expression);
        assert!(
            matches!(operation!(*root), Some(Operation::Call)),
            "{:?}",
            operation!(*root)
        );
        let left = left!(*root).unwrap();
        assert!(
            matches!(**left, Expression::Primitive(Value::Identifier { name: _ })),
            "{:?}",
            **left
        );
        let right = right!(*root).unwrap();
        assert!(
            matches!(operation!(**right), Some(Operation::Comma)),
            "{:?}",
            operation!(**right)
        );
    }

    #[test]
    fn test_function_call_no_args() {
        let arena = Bump::new();
        let input = "fn()";
        let expression = CParser::parse(Rule::expression, input).unwrap();
        let root = parse_expression(&arena, expression);
        assert!(
            matches!(operation!(*root), Some(Operation::Call)),
            "{:?}",
            *root
        );
        assert!(matches!(*root, Expression::Unary(_)), "{:?}", *root);
    }

    #[test]
    fn test_array_index() {
        let arena = Bump::new();
        let input = "arr[1]";
        let expression = CParser::parse(Rule::expression, input).unwrap();
        let root = parse_expression(&arena, expression);
        assert!(
            matches!(operation!(*root), Some(Operation::Index)),
            "{:?}",
            operation!(*root)
        );
        let left = left!(*root).unwrap();
        assert!(
            matches!(**left, Expression::Primitive(Value::Identifier { name: _ })),
            "{:?}",
            **left
        );
        let right = right!(*root).unwrap();
        assert!(
            matches!(**right, Expression::Primitive(Value::Constant { value: _ })),
            "{:?}",
            **right
        );
    }
}
