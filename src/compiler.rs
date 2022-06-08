use crate::common::{Core, Op};
use crate::value::{Function, Value};

pub struct Compiler {
    pub function: Function,
    locals: Vec<(String, usize)>, // (Name, Depth)
    scope_depth: usize,
    continues: Vec<Vec<usize>>,
    breaks: Vec<Vec<usize>>,
}

impl Compiler {
    pub fn new() -> Compiler {
        Compiler {
            function: Function::new(0),
            locals: vec![],
            scope_depth: 0,
            continues: vec![],
            breaks: vec![],
        }
    }

    fn arithmetic_op(&self, x: &Core) -> Option<Op> {
        if let Core::Get(x) = x {
            return Some(match x.as_str() {
                "+" => Op::Add,
                "-" => Op::Subtract,
                "*" => Op::Multiply,
                "/" => Op::Divide,
                _ => return None,
            });
        }
        None
    }

    fn resolve_local(&mut self, name: &String) -> Option<usize> {
        for i in (0..self.locals.len()).rev() {
            let (n, _) = &self.locals[i];
            if n == name {
                return Some(i);
            }
        }
        None
    }

    fn resolve_global(&mut self, name: &String) -> Option<usize> {
        for i in 0..self.function.chunk.constants.len() {
            if let Value::Str(x) = &self.function.chunk.constants[i] {
                if x == name {
                    return Some(i);
                }
            }
        }
        None
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;

        while let Some(x) = self.locals.last() {
            if x.1 <= self.scope_depth {
                break;
            }
            self.locals.pop();
            self.function.chunk.code.push(Op::Pop as u8);
        }
    }

    fn declare_var(&mut self, name: &String) {
        // TODO; Check if local already exists
        if self.scope_depth > 0 {
            self.add_local(name);
        }
    }

    fn define_var(&mut self, name: &String) {
        if self.scope_depth == 0 {
            self.function.chunk.code.push(Op::SetGlobal as u8);
            let name_idx = self.function.chunk.add_constant(Value::Str(name.clone()));
            self.function.chunk.code.push(name_idx as u8);
        }
    }

    fn add_local(&mut self, name: &String) {
        self.locals.push((name.clone(), self.scope_depth))
    }

    pub fn compile(&mut self, expr: &Core) {
        match expr {
            Core::Lit(x) => {
                let idx = self.function.chunk.add_constant(x.clone()) as u8;
                self.function.chunk.code.push(Op::LoadConstant as u8);
                self.function.chunk.code.push(idx);
            }

            Core::Lambda(args, body) => {
                let mut cc = Compiler::new();
                cc.begin_scope(); // Entire Function Body is Local
                for arg in args {
                    cc.declare_var(arg);
                    cc.define_var(arg);
                }
                cc.compile(body);
                cc.done();
                let f = Value::Closure(cc.function);
                let idx = self.function.chunk.add_constant(f) as u8;
                self.function.chunk.code.push(Op::LoadConstant as u8);
                self.function.chunk.code.push(idx);
            }

            Core::Call(name, args) => {
                for arg in args {
                    self.compile(arg);
                }

                if let Some(op) = self.arithmetic_op(name) {
                    self.function.chunk.code.push(op as u8);
                } else {
                    self.compile(name);
                    self.function.chunk.code.push(Op::Call as u8);
                    self.function.chunk.code.push(args.len() as u8);
                }
            }

            Core::Return(expr) => {
                self.compile(expr);
                self.function.chunk.code.push(Op::Return as u8);
            }

            // Variable Access
            Core::Let(name, value) => {
                self.compile(value);
                self.declare_var(name);
                self.define_var(name);
            }

            Core::Get(name) => {
                if let Some(i) = self.resolve_local(name) {
                    self.function.chunk.code.push(Op::GetLocal as u8);
                    self.function.chunk.code.push(i as u8);
                } else {
                    let idx = self.function.chunk.add_constant(Value::Str(name.clone())) as u8;
                    self.function.chunk.code.push(Op::GetGlobal as u8);
                    self.function.chunk.code.push(idx);
                }
            }

            Core::Set(name, value) => {
                self.compile(value);

                if let Some(idx) = self.resolve_local(name) {
                    self.function.chunk.code.push(Op::SetLocal as u8);
                    self.function.chunk.code.push(idx as u8);
                } else if let Some(idx) = self.resolve_global(name) {
                    self.function.chunk.code.push(Op::SetGlobal as u8);
                    self.function.chunk.code.push(idx as u8);
                } else {
                    panic!("Global not defined")
                }
            }

            Core::Block(exprs) => {
                self.begin_scope();
                for expr in exprs.iter() {
                    self.compile(expr);
                }
                self.end_scope();
            }

            Core::If(condition, on_true, on_false) => {
                // TODO: Implement break in If and Block

                self.compile(condition);

                let then_jump_idx = self.function.chunk.code.len();
                self.function.chunk.code.push(Op::JumpIfFalse as u8);
                self.function.chunk.code.push(0xff);
                self.function.chunk.code.push(0xff);

                self.compile(on_true);

                let then_end_jump_idx = self.function.chunk.code.len();
                self.function.chunk.code.push(Op::RelJump as u8);
                self.function.chunk.code.push(0xff);
                self.function.chunk.code.push(0xff);

                self.function.chunk.write_byte_double(
                    then_jump_idx + 1,
                    self.function.chunk.code.len() - then_jump_idx,
                );

                self.compile(on_false);

                self.function.chunk.write_byte_double(
                    then_end_jump_idx + 1,
                    self.function.chunk.code.len() - then_end_jump_idx,
                );
            }

            Core::Loop(expr) => {
                let loop_start_idx = self.function.chunk.code.len();
                self.continues.push(vec![]);
                self.breaks.push(vec![]);

                self.compile(expr);

                self.function.chunk.code.push(Op::AbsJump as u8);
                self.function.chunk.code.push(0xff);
                self.function.chunk.code.push(0xff);

                self.function
                    .chunk
                    .write_byte_double(self.function.chunk.code.len() - 2, loop_start_idx);

                let loop_exit_idx = self.function.chunk.code.len();

                for continue_jump_idx in self.continues.pop().unwrap().iter() {
                    self.function
                        .chunk
                        .write_byte_double(continue_jump_idx + 1, loop_start_idx);
                }

                for break_jump_idx in self.breaks.pop().unwrap().iter() {
                    self.function
                        .chunk
                        .write_byte_double(break_jump_idx + 1, loop_exit_idx);
                }
            }

            Core::Continue => {
                let continue_jump_idx = self.function.chunk.code.len();
                self.function.chunk.code.push(Op::AbsJump as u8);
                self.function.chunk.code.push(0xff);
                self.function.chunk.code.push(0xff);
                let k = self.continues.len() - 1;
                self.continues[k].push(continue_jump_idx);
            }

            Core::Break => {
                let break_jump_idx = self.function.chunk.code.len();
                self.function.chunk.code.push(Op::AbsJump as u8);
                self.function.chunk.code.push(0xff);
                self.function.chunk.code.push(0xff);
                let k = self.breaks.len() - 1;
                self.breaks[k].push(break_jump_idx);
            }
            _ => todo!("Not implemented yet"),
        }
    }

    pub fn done(&mut self) {
        self.compile(&Core::Lit(Value::Bool(false)));
        self.function.chunk.code.push(Op::Return as u8);
    }
}
