use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

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
use crate::value::{Closure, Function, Value};
use crate::vm::VM;

fn repl(ctx: &ParserContext, ffi: &FFI, dbg: bool) {
    let stdin = io::stdin();
    loop {
        let line = {
            print!(">> ");
            io::stdout().flush().unwrap();
            let mut iter = stdin.lock().lines();
            iter.next().unwrap().unwrap()
        };

        run("".to_string(), line, &ctx, &ffi, dbg);
    }
}

fn run(fname: String, content: String, ctx: &ParserContext, ffi: &FFI, dbg: bool) {
    if fname.len() > 0 {
        println!("Running {}", fname);
        println!("---");
    }

    let ts = lex(content);
    if dbg {
        println!("Tokens: {:?}", ts);
    }
    let mut lower_parser = LowerParser::new(ts);
    let expr = lower_parser.parse();
    if dbg {
        println!("Low Parse: {:?}", expr);
    }

    let mut higher_parser = HigherParser::new(vec![expr], &ctx);
    let core_expr = higher_parser.parse();
    if dbg {
        println!("High Parse: {:?}", core_expr);
    }

    let mut cc = Compiler::new(dbg);
    cc.compile(&core_expr);
    let f = cc.ctxs[0].function.clone();

    let mut vm = VM::new(Closure::new(f), &ffi);
    vm.run(dbg);
}

fn main() {
    let mut ffi = FFI::new();
    ffi.insert(
        "print".to_string(),
        Box::new(|x| {
            println!("{}", x);
            Value::Bool(false)
        }),
    );

    ffi.insert(
        "exit".to_string(),
        Box::new(|_| {
            println!("exiting");
            std::process::exit(0);
        }),
    );

    ffi.insert(
        "time".to_string(),
        Box::new(|_| {
            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            Value::Float((since_the_epoch.as_millis() as f64) * 0.001)
        }),
    );

    let infix_ops = vec![
        "".to_string(),
        "/".to_string(),
        "*".to_string(),
        "-".to_string(),
        "+".to_string(),
        "==".to_string(),
        "->".to_string(),
        "=".to_string(),
    ];

    let mut prefix_macros = HashMap::new();
    let mut infix_macros = HashMap::new();

    // Prefix Macros
    let prefix_return_macro: MacroRulePrefix =
        Box::new(|ctx, expr| Core::Return(Box::new(HigherParser::new(expr.clone(), ctx).parse())));

    let prefix_break_macro: MacroRulePrefix = Box::new(|_, _| Core::Break);

    let prefix_continue_macro: MacroRulePrefix = Box::new(|_, _| Core::Continue);

    let prefix_if_macro: MacroRulePrefix =
        // If cond then on_true;
        // If cond then on_true else on_true;
        Box::new(|ctx, body| {
            if body.len() == 3 || body.len() == 5 {
                if let Expr::Name(n) = &body[1] {
                    let cond = &body[0];
                    assert_eq!(n, &"then".to_string());
                    let on_true = &body[2];
                    let mut on_false = &Expr::LitInt(0);
                    if body.len() == 5 {
                        if let Expr::Name(n) = &body[3] {
                            assert_eq!(n, &"else".to_string());
                            on_false = &body[4];
                        }
                    }
                    return Core::If(
                        Box::new(HigherParser::new(vec![cond.clone()], ctx).parse()),
                        Box::new(HigherParser::new(vec![on_true.clone()], ctx).parse()),
                        Box::new(HigherParser::new(vec![on_false.clone()], ctx).parse()),
                    )
                }
            }
            todo!()
        });

    let prefix_loop_macro: MacroRulePrefix =
        Box::new(|ctx, body| Core::Loop(Box::new(HigherParser::new(body.clone(), ctx).parse())));

    // Infix Macros
    let infix_lambda_macro: MacroRuleInfix = Box::new(|op, ctx, args, body| {
        Core::Lambda(
            args.iter()
                .map(|x| match x {
                    Expr::Name(n) => n.clone(),
                    _ => todo!(),
                })
                .collect(),
            Box::new(Core::Block(vec![
                HigherParser::new(body.clone(), ctx).parse()
            ])),
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

    prefix_macros.insert("return".to_string(), prefix_return_macro);
    prefix_macros.insert("continue".to_string(), prefix_continue_macro);
    prefix_macros.insert("break".to_string(), prefix_break_macro);
    prefix_macros.insert("if".to_string(), prefix_if_macro);
    prefix_macros.insert("loop".to_string(), prefix_loop_macro);

    infix_macros.insert("->".to_string(), infix_lambda_macro);
    infix_macros.insert("=".to_string(), infix_assign_macro);

    let ctx = ParserContext::new(&infix_ops, &infix_macros, &prefix_macros);

    let mut files = vec![];
    let mut dbg = false;
    for x in env::args().skip(1) {
        if x == "-d".to_string() || x == "--debug".to_string() {
            dbg = true;
        } else {
            files.push((x.clone(), fs::read_to_string(x).expect("can't read file.")));
        }
    }

    if files.len() == 0 {
        repl(&ctx, &ffi, dbg);
    } else {
        for (name, content) in files {
            run(name, content, &ctx, &ffi, dbg)
        }
    }

    // Sum till n
    // let f = n -> {let s = 0; loop {if (n == 0) then (return s) else {s = s + n; n = n - 1}}}; print (f 20);
    // let s = n -> if (n == 0) then (return 0) else (return (n + (s (n - 1)))); print (s 20);
    // Factorial
    // let f = n -> if (n == 0) then (return 1) else (return (n * (f (n - 1)))); print (f 20);
    // fibonacci
    // let f = n -> if (n == 0) then (return 1) else (if (n == 1) then (return 1) else (return (f (n-1) + f (n-2)))); print (f 20);
    // Funcy stuff
    // let cons = x y -> (return (f -> (return (f x y)))); print ((cons 1 2) (x y -> return x));
}
