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
use crate::parser::{HigherParser, LowerParser};
use crate::value::Value;
use crate::vm::VM;

fn repl(infix_ops: Vec<String>, ffi: FFI) {
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
            &infix_ops
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
        "=".to_string()
    ];
    repl(infix_ops, ffi);
}
