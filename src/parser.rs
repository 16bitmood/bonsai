use crate::common::Core;
use crate::lexer::Tk;
use crate::value::Value;


#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    FExpr(Vec<Expr>),
    Tuple(Vec<Expr>),
    List(Vec<Expr>),
    Block(Vec<Expr>),

    Name(String),
    NameInfix(String),
    LitStr(String),
    LitFloat(f64),
    LitInt(isize),
}

pub struct LowerParser {
    tokens: Vec<Tk>,
    current: usize,
}

impl LowerParser {
    pub fn new(ts: Vec<Tk>) -> LowerParser {
        LowerParser {
            tokens: ts,
            current: 0,
        }
    }

    fn advance(&mut self) -> Option<&Tk> {
        if self.tokens.len() >= self.current {
            self.current += 1;
            Some(&self.tokens[self.current - 1])
        } else {
            None
        }
    }

    pub fn parse(&mut self) -> Expr {
        self.list_expr(Tk::Semicolon, Tk::Eof, true)
    }

    pub fn list_expr(&mut self, sep: Tk, end: Tk, newline_is_sep: bool) -> Expr {
        let mut list: Vec<Expr> = vec![];
        let mut elem: Vec<Expr> = vec![];

        while let Some(t) = self.advance() {
            match t {
                t if (*t == end) => {
                    if elem.len() != 0 {
                        if elem.len() != 1 {
                            list.push(Expr::FExpr(elem));
                        } else {
                            list.push(elem[0].clone());
                        }
                    };

                    if end == Tk::RParen && list.len() == 1 {
                        return list[0].clone(); // (a b c) is grouping (a, b, c) is tuple
                    } else {
                        return match end {
                            Tk::RParen => Expr::Tuple(list),
                            Tk::RSquare => Expr::List(list),
                            Tk::RBrace | Tk::Eof => Expr::Block(list),
                            _ => panic!("Now what."),
                        };
                    }
                }

                t if (*t == sep) || sep == Tk::Eof => {
                    if elem.len() == 1 {
                        list.push(elem[0].clone());
                    } else {
                        list.push(Expr::FExpr(elem));
                    }
                    elem = vec![];
                }

                Tk::NewLine if newline_is_sep => {
                    if elem.len() == 1 {
                        list.push(elem[0].clone());
                    } else {
                        list.push(Expr::FExpr(elem));
                    }
                    elem = vec![];
                }

                Tk::LBrace => elem.push(self.list_expr(Tk::Semicolon, Tk::RBrace, true)),

                Tk::LParen => elem.push(self.list_expr(Tk::Comma, Tk::RParen, true)),

                Tk::LSquare => elem.push(self.list_expr(Tk::Comma, Tk::RSquare, false)),

                Tk::LitInt(n) => elem.push(Expr::LitInt(*n)),
                Tk::LitFloat(n) => elem.push(Expr::LitFloat(*n)),
                Tk::LitStr(s) => elem.push(Expr::LitStr(s.clone())),

                Tk::Name(n) => elem.push(Expr::Name(n.clone())),
                Tk::NameInfix(n) => elem.push(Expr::NameInfix(n.clone())),

                _ => panic!("Unexpected Tk!{:?}", t),
            };
        }
        panic!("Unreachable")
    }
}

pub struct HigherParser<'a> {
    fexpr: Vec<Expr>,
    current_idx: usize,
    infix_operators: &'a Vec<String>,
}

// TODO: Unnecessarily Complex?
impl HigherParser<'_> {
    pub fn new(fexpr: Vec<Expr>, ops: &Vec<String>) -> HigherParser {
        HigherParser {
            fexpr: fexpr,
            current_idx: 0,
            infix_operators: ops,
        }
    }

    fn peek(&self) -> Option<&Expr> {
        self.fexpr.get(self.current_idx)
    }

    fn advance(&mut self) {
        self.current_idx += 1;
    }

    fn check_infix(&self, op_id: usize) -> bool {
        if let Some(Expr::NameInfix(y)) = self.peek() {
            &self.infix_operators[op_id] == y
        } else {
            false
        }
    }

    pub fn parse(&mut self) -> Core {
        self.parse_infix(self.infix_operators.len() - 1)
    }

    fn parse_infix(&mut self, op_id: usize) -> Core {
        if op_id == 0 {
            return self.parse_prefix();
        } // Else check if macro, then apply rules.

        let mut left = self.parse_infix(op_id - 1);
        while self.check_infix(op_id) {
            self.advance();


            let right = self.parse_infix(op_id - 1);

            if self.infix_operators[op_id].as_str() == "=" {
                if let Core::Get(x) = left {
                    left = Core::Let(
                        x,
                        Box::new(right)
                    )
                } else {
                    todo!("Invalid set");
                }
            } else if self.infix_operators[op_id].as_str() == "->" {
                if let Core::Get(x) = left {
                    left = Core::Lambda(
                        vec![x],
                        Box::new(right)
                    )
                } else if let Core::Call(x, args) = left {
                    let mut args = args.clone();
                    args.insert(0 , *x);
                    left = Core::Lambda(
                        args.iter().map(|x| {
                            if let Core::Get(y) = x {
                                y.clone()
                            } else {
                                todo!()
                            }
                        }).collect(),
                        Box::new(Core::Block(vec![right]))
                    )
                } else {
                    todo!("Invalid Lambda")
                }
            } else {
                left = Core::Call(
                    Box::new(Core::Get(self.infix_operators[op_id].clone())),
                    vec![left, right],
                );
            }
        }
        left
    }

    pub fn parse_prefix(&mut self) -> Core {
        let mut fcall = vec![];
        while let Some(x) = self.peek() {
            let x = x.clone();
            let arg = match x {
                Expr::LitStr(s) => Core::Lit(Value::Str(s.clone())),
                Expr::LitFloat(f) => Core::Lit(Value::Float(f)),
                Expr::LitInt(i) => Core::Lit(Value::Int(i)),

                Expr::FExpr(xs) => HigherParser::new(xs, self.infix_operators).parse(),

                Expr::Block(xs) => {
                    let mut block = vec![];
                    for x in xs {
                        block.push(HigherParser::new(vec![x], self.infix_operators).parse());
                    }
                    Core::Block(block)
                }

                Expr::Name(n) => Core::Get(n.clone()),

                Expr::NameInfix(_) => break,

                _ => todo!(),
            };
            self.advance();
            fcall.push(arg);
        }

        if fcall.len() == 1 {
            fcall[0].clone()
        } else {
            Core::Call(
                Box::new(fcall[0].clone()),
                fcall.iter().skip(1).map(|x| x.clone()).collect(),
            )
        }
    }
}
