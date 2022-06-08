use std::collections::HashMap;
use std::io::{self, BufRead, Write};

mod common;
mod compiler;
mod config;
mod lexer;
mod native;
mod parser;
mod value;
mod vm;

use crate::common::{Chunk, Core, Op};
use crate::compiler::Compiler;
use crate::lexer::lex;
use crate::native::FFI;
use crate::parser::{
    Expr, HigherParser, LowerParser, MacroRuleInfix, MacroRulePrefix, ParserContext,
};
use crate::value::Value;
use crate::vm::VM;

fn repl(ctx: ParserContext, ffi: FFI) {
    let stdin = io::stdin();
    let mut iter = stdin.lock().lines();
    loop {
        print!(">> ");
        io::stdout().flush().unwrap();
        let line = iter.next().unwrap().unwrap();

        let ts = lex(line);
        println!("Tokens: {:?}", ts);

        let mut lower_parser = LowerParser::new(ts);
        let expr = lower_parser.parse();
        println!("Low Parse: {:?}", expr);

        let mut higher_parser = HigherParser::new(
            vec![expr],
            &ctx, // &infix_ops,
                  // &prefix_macros,
                  // &infix_macros
        );
        let core_expr = higher_parser.parse();
        println!("High Parse: {:?}", core_expr);

        let mut cc = Compiler::new();
        cc.compile(&core_expr);
        cc.done();

        let function = cc.function.clone();
        function.chunk.disassemble();

        let mut vm = VM::new(function, &ffi);
        vm.run();
    }
}

fn main() {
    let mut ffi = FFI::new();
    ffi.insert(
        "print".to_string(),
        Box::new(|x| {
            println!("{:?}", x);
            Value::Bool(false)
        }),
    );

    ffi.insert(
        "exit".to_string(),
        Box::new(|x| {
            println!("exiting {}", x);
            std::process::exit(0);
        }),
    );

    let infix_ops = vec![
        "".to_string(),
        "/".to_string(),
        "*".to_string(),
        "-".to_string(),
        "+".to_string(),
        "->".to_string(),
        "=".to_string(),
    ];

    let mut prefix_macros = HashMap::new();
    let mut infix_macros = HashMap::new();

    let infix_lambda_macro: MacroRuleInfix = Box::new(|op, ctx, args, body| {
        Core::Lambda(
            args.iter()
                .map(|x| match x {
                    Expr::Name(n) => n.clone(),
                    _ => todo!(),
                })
                .collect(),
            Box::new(HigherParser::new(body.clone(), ctx).parse()),
        )
    });

    let infix_assign_macro: MacroRuleInfix = Box::new(|op, ctx, vars, value| {
        if vars.len() > 2 {
            todo!()
        } else if let Expr::Name(n) = vars.last().unwrap() {
            let value = Box::new(HigherParser::new(value.clone(), ctx).parse());
            if vars.len() == 1 {
                Core::Set(n.clone(), value)
            } else if let Expr::Name(l) = &vars[0] {
                assert_eq!(l, "let");
                Core::Let(n.clone(), value)
            } else {
                todo!()
            }
        } else {
            todo!()
        }
    });

    let prefix_return_macro: MacroRulePrefix =
        Box::new(|ctx, expr| Core::Return(Box::new(HigherParser::new(expr.clone(), ctx).parse())));

    infix_macros.insert("->".to_string(), infix_lambda_macro);
    infix_macros.insert("=".to_string(), infix_assign_macro);

    prefix_macros.insert("return".to_string(), prefix_return_macro);

    let ctx = ParserContext::new(&infix_ops, &infix_macros, &prefix_macros);

    repl(ctx, ffi);
}
