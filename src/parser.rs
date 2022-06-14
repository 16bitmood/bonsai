use std::collections::HashMap;

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

pub struct ParserContext<'a> {
    infix_operators: &'a Vec<String>,
    infix_macros: &'a HashMap<String, MacroRuleInfix>,
    prefix_macros: &'a HashMap<String, MacroRulePrefix>,
}

impl ParserContext<'_> {
    pub fn new<'a>(
        infix_operators: &'a Vec<String>,
        infix_macros: &'a HashMap<String, MacroRuleInfix>,
        prefix_macros: &'a HashMap<String, MacroRulePrefix>,
    ) -> ParserContext<'a> {
        ParserContext {
            infix_operators,
            infix_macros,
            prefix_macros,
        }
    }
}

pub type MacroRulePrefix = Box<dyn Fn(&ParserContext, &Vec<Expr>) -> Core>;
pub type MacroRuleInfix = Box<dyn Fn(usize, &ParserContext, &Vec<Expr>, &Vec<Expr>) -> Core>;

pub struct HigherParser<'a> {
    fexpr: Vec<Expr>,
    current_idx: usize,
    ctx: &'a ParserContext<'a>,
}

// TODO: Unnecessarily Complex?
impl HigherParser<'_> {
    pub fn new<'a>(fexpr: Vec<Expr>, ctx: &'a ParserContext) -> HigherParser<'a> {
        HigherParser {
            fexpr: fexpr,
            current_idx: 0,
            ctx: ctx,
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
            &self.ctx.infix_operators[op_id] == y
        } else {
            false
        }
    }

    fn check_infix_till_end(&self, op_id: usize) -> bool {
        for i in self.current_idx..self.fexpr.len() {
            if let Expr::NameInfix(y) = &self.fexpr[i] {
                if &self.ctx.infix_operators[op_id] == y {
                    return true;
                }
            }
        }
        false
    }

    pub fn parse(&mut self) -> Core {
        self.parse_infix(self.ctx.infix_operators.len() - 1)
    }

    fn take_till_infix(&mut self, op_id: usize) -> Vec<Expr> {
        let mut xs = vec![];
        while !self.check_infix(op_id) {
            if self.peek() == None {
                return xs;
            }
            xs.push(self.peek().unwrap().clone());
            self.advance();
        }
        xs
    }

    fn parse_infix(&mut self, op_id: usize) -> Core {
        if op_id == 0 {
            return self.parse_prefix();
        } else if self
            .ctx
            .infix_macros
            .contains_key(&self.ctx.infix_operators[op_id])
            && self.check_infix_till_end(op_id)
        {
            let flat_left = self.take_till_infix(op_id);
            if self.check_infix(op_id) {
                self.advance();
                let flat_right = self.take_till_infix(op_id);
                return self
                    .ctx
                    .infix_macros
                    .get(&self.ctx.infix_operators[op_id])
                    .unwrap()(op_id, self.ctx, &flat_left, &flat_right);
            } else {
                todo!()
            }
        }

        let mut left = self.parse_infix(op_id - 1);
        while self.check_infix(op_id) {
            self.advance();

            let right = self.parse_infix(op_id - 1);

            left = Core::Call(
                Box::new(Core::Get(self.ctx.infix_operators[op_id].clone())),
                vec![left, right],
            );
        }
        left
    }

    pub fn parse_prefix(&mut self) -> Core {
        if let Some(Expr::Name(x)) = self.peek() {
            if self.ctx.prefix_macros.contains_key(x) {
                return self.ctx.prefix_macros.get(x).unwrap()(
                    self.ctx,
                    &self.fexpr.iter().skip(1).map(|x| x.clone()).collect(),
                );
            }
        }

        let mut fcall = vec![];
        while let Some(x) = self.peek() {
            let x = x.clone();
            let arg = match x {
                Expr::LitStr(s) => Core::Lit(Value::Str(s.clone())),
                Expr::LitFloat(f) => Core::Lit(Value::Float(f)),
                Expr::LitInt(i) => Core::Lit(Value::Int(i)),

                Expr::FExpr(xs) => HigherParser::new(xs, self.ctx).parse(),

                Expr::Block(xs) => {
                    let mut block = vec![];
                    for x in xs {
                        block.push(HigherParser::new(vec![x], self.ctx).parse());
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
