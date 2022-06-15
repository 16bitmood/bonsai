use crate::common::{Chunk, Core, Op};
use crate::value::{Closure, Function, Value};

#[derive(Debug, Clone, Copy)]
pub enum Upvalue {
    Local(usize),
    NonLocal(usize),
}

pub struct CCtx {
    pub function: Function,
    locals: Vec<(String, usize, bool)>, // (Name, Depth, isCaptured)
    upvalues: Vec<Upvalue>,
    scope_depth: usize,
    continues: Vec<Vec<usize>>,
    breaks: Vec<Vec<usize>>,
}

impl CCtx {
    pub fn new() -> CCtx {
        CCtx {
            function: Function::new(0, 0, Chunk::new(vec![], vec![])),
            locals: vec![],
            upvalues: vec![],
            scope_depth: 0,
            continues: vec![],
            breaks: vec![],
        }
    }
}

pub struct Compiler {
    pub ctxs: Vec<CCtx>,
    current: usize,
    dbg: bool
}

fn try_arithmetic_op(x: &Core) -> Option<Op> {
    if let Core::Get(x) = x {
        return Some(match x.as_str() {
            "==" => Op::IsEqual,
            "+" => Op::Add,
            "-" => Op::Subtract,
            "*" => Op::Multiply,
            "/" => Op::Divide,
            _ => return None,
        });
    }
    None
}

impl Compiler {
    pub fn new(dbg: bool) -> Compiler {
        Compiler {
            ctxs: vec![CCtx::new()],
            current: 0,
            dbg
        }
    }

    #[inline]
    fn add_constant(&mut self, x: Value) -> usize {
        self.ctxs[self.current].function.chunk.add_constant(x)
    }

    #[inline]
    fn add_byte(&mut self, b: u8) {
        self.ctxs[self.current].function.chunk.code.push(b);
    }

    #[inline]
    fn add_bytes(&mut self, b1: u8, b2: u8) {
        self.add_byte(b1);
        self.add_byte(b2);
    }

    fn resolve_local(&self, name: &String, ctx_i: usize) -> Option<usize> {
        let locals = &self.ctxs[ctx_i].locals;
        for i in (0..locals.len()).rev() {
            let (n, _, _) = &locals[i];
            if n == name {
                return Some(i);
            }
        }
        None
    }

    fn resolve_upvalue(&mut self, name: &String, ctx_i: usize) -> Option<usize> {
        if self.ctxs.len() <= 1 {
            return None;
        } else if let Some(idx) = self.resolve_local(name, ctx_i - 1) {
            self.ctxs[ctx_i - 1].locals[idx].2 = true;
            Some(self.add_upvalue(Upvalue::Local(idx), ctx_i))
        } else if let Some(idx) = self.resolve_upvalue(name, ctx_i - 1) {
            Some(self.add_upvalue(Upvalue::NonLocal(idx), ctx_i))
        } else {
            None
        }
    }

    fn resolve_global(&self, name: &String) -> Option<usize> {
        let consts = &self.ctxs[self.current].function.chunk.constants;
        for i in 0..consts.len() {
            if let Value::Str(x) = &consts[i] {
                if x == name {
                    return Some(i);
                }
            }
        }
        None
    }

    fn add_local(&mut self, name: &String, ctx_i: usize) {
        let depth = self.ctxs[ctx_i].scope_depth;
        self.ctxs[ctx_i].locals.push((name.clone(), depth, false))
    }

    fn add_upvalue(&mut self, up_insert: Upvalue, ctx_i: usize) -> usize {
        for i in 0..self.ctxs[ctx_i].upvalues.len() {
            let up_found = self.ctxs[ctx_i].upvalues[i];
            match (up_found, up_insert) {
                (Upvalue::Local(x), Upvalue::Local(y))
                | (Upvalue::NonLocal(x), Upvalue::NonLocal(y)) => {
                    if x == y {
                        return i;
                    }
                }
                _ => continue,
            }
        }
        self.ctxs[ctx_i].function.upvalue_count += 1;
        self.ctxs[ctx_i].upvalues.push(up_insert);
        return self.ctxs[ctx_i].function.upvalue_count - 1;
    }

    fn begin_scope(&mut self) {
        self.ctxs[self.current].scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.ctxs[self.current].scope_depth -= 1;

        while let Some(x) = self.ctxs[self.current].locals.last() {
            if x.1 <= self.ctxs[self.current].scope_depth {
                break;
            }
            self.add_byte(Op::Pop as u8);
            self.ctxs[self.current].locals.pop();
        }
    }

    fn declare_var(&mut self, name: &String) {
        // TODO; Check if local already exists
        if self.ctxs[self.current].scope_depth > 0 {
            self.add_local(name, self.current);
        }
    }

    fn define_var(&mut self, name: &String) {
        if self.ctxs[self.current].scope_depth == 0 {
            self.ctxs[self.current]
                .function
                .chunk
                .code
                .push(Op::SetGlobal as u8);
            let name_idx = self.ctxs[self.current]
                .function
                .chunk
                .add_constant(Value::Str(name.clone()));
            self.ctxs[self.current]
                .function
                .chunk
                .code
                .push(name_idx as u8);
        }
    }

    pub fn compile(&mut self, expr: &Core) {
        match expr {
            Core::Lit(x) => {
                let idx = self.add_constant(x.clone()) as u8;
                self.add_bytes(Op::LoadConstant as u8, idx);
            }

            Core::Lambda(args, body) => {
                let sub_ctx = {
                    self.ctxs.push(CCtx::new());
                    self.current += 1;

                    self.ctxs[self.current].function.arity = args.len();
                    self.begin_scope();
                    for arg in args {
                        self.declare_var(arg);
                        self.define_var(arg);
                    }
                    self.compile(body);
                    self.done();

                    self.current -= 1;
                    self.ctxs.pop().unwrap()
                };

                let function = sub_ctx.function;
                let upvalues = sub_ctx.upvalues;

                let f = Value::Function(function);
                let idx = self.add_constant(f) as u8;

                self.add_bytes(Op::MakeClosure as u8, idx);

                for up in upvalues {
                    match up {
                        Upvalue::Local(x) => {
                            self.add_bytes(true as u8, x as u8);
                        }
                        Upvalue::NonLocal(x) => {
                            self.add_bytes(false as u8, x as u8);
                        }
                    }
                }
            }

            Core::Call(name, args) => {
                for arg in args {
                    self.compile(arg);
                }

                if let Some(op) = try_arithmetic_op(name) {
                    self.add_byte(op as u8);
                } else {
                    self.compile(name);
                    self.add_bytes(Op::Call as u8, args.len() as u8);
                }
            }

            Core::Return(expr) => {
                self.compile(expr);
                self.add_byte(Op::Return as u8);
            }

            // Variable Access
            Core::Let(name, value) => {
                self.declare_var(name);
                self.compile(value);
                self.define_var(name);
            }

            Core::Get(name) => {
                if let Some(idx) = self.resolve_local(name, self.current) {
                    self.add_bytes(Op::GetLocal as u8, idx as u8);
                } else if let Some(idx) = self.resolve_upvalue(name, self.current) {
                    self.add_bytes(Op::GetUpvalue as u8, idx as u8);
                } else {
                    let idx = self.add_constant(Value::Str(name.clone())) as u8;
                    self.add_bytes(Op::GetGlobal as u8, idx as u8);
                }
            }

            Core::Set(name, value) => {
                self.compile(value);

                if let Some(idx) = self.resolve_local(name, self.current) {
                    self.add_bytes(Op::SetLocal as u8, idx as u8);
                } else if let Some(idx) = self.resolve_upvalue(name, self.current) {
                    self.add_bytes(Op::SetUpvalue as u8, idx as u8);
                } else if let Some(idx) = self.resolve_global(name) {
                    self.add_bytes(Op::SetGlobal as u8, idx as u8);
                } else {
                    panic!("Global not defined")
                }
            }

            Core::Block(exprs) => {
                self.begin_scope();

                for expr in exprs.iter() {
                    self.compile(expr);
                }
                // for (i, expr) in exprs.iter().enumerate() {
                //     if self.compile(expr) && !(i == exprs.len() - 1) {
                //         // self.add_byte(Op::Pop as u8);
                //     }
                // }
                self.end_scope();
            }

            Core::If(condition, on_true, on_false) => {
                // TODO: Implement break in If and Block

                self.compile(condition);

                let then_jump_idx = self.ctxs[self.current].function.chunk.code.len();
                self.add_byte(Op::JumpIfFalse as u8);
                self.add_bytes(0xff, 0xff);

                self.compile(on_true);

                let then_end_jump_idx = self.ctxs[self.current].function.chunk.code.len();

                self.add_byte(Op::Jump as u8);
                self.add_bytes(0xff, 0xff);

                let k = self.ctxs[self.current].function.chunk.code.len() - then_jump_idx;
                self.ctxs[self.current]
                    .function
                    .chunk
                    .write_byte_double(then_jump_idx + 1, k);

                self.compile(on_false);

                let k = self.ctxs[self.current].function.chunk.code.len() - then_end_jump_idx;
                self.ctxs[self.current]
                    .function
                    .chunk
                    .write_byte_double(then_end_jump_idx + 1, k);
            }

            Core::Loop(expr) => {
                let loop_start_idx = self.ctxs[self.current].function.chunk.code.len();
                self.ctxs[self.current].continues.push(vec![]);
                self.ctxs[self.current].breaks.push(vec![]);

                self.compile(expr);

                // Begin Scope here
                // Pop Here?
                self.add_byte(Op::AbsJump as u8);
                self.add_bytes(0xff, 0xff);

                let k = self.ctxs[self.current].function.chunk.code.len() - 2;
                self.ctxs[self.current]
                    .function
                    .chunk
                    .write_byte_double(k, loop_start_idx);

                let loop_exit_idx = self.ctxs[self.current].function.chunk.code.len();

                for continue_jump_idx in self.ctxs[self.current].continues.pop().unwrap().iter() {
                    self.ctxs[self.current]
                        .function
                        .chunk
                        .write_byte_double(continue_jump_idx + 1, loop_start_idx);
                }

                for break_jump_idx in self.ctxs[self.current].breaks.pop().unwrap().iter() {
                    self.ctxs[self.current]
                        .function
                        .chunk
                        .write_byte_double(break_jump_idx + 1, loop_exit_idx);
                }
            }

            Core::Continue => {
                let continue_jump_idx = self.ctxs[self.current].function.chunk.code.len();
                self.add_byte(Op::AbsJump as u8);
                self.add_bytes(0xff, 0xff);
                let k = self.ctxs[self.current].continues.len() - 1;
                self.ctxs[self.current].continues[k].push(continue_jump_idx);
            }

            Core::Break => {
                let break_jump_idx = self.ctxs[self.current].function.chunk.code.len();
                self.add_byte(Op::AbsJump as u8);
                self.add_bytes(0xff, 0xff);
                let k = self.ctxs[self.current].breaks.len() - 1;
                self.ctxs[self.current].breaks[k].push(break_jump_idx);
            }
        }
    }

    pub fn done(&mut self) -> Function {
        self.compile(&Core::Lit(Value::None));
        self.add_byte(Op::Return as u8);

        if self.dbg {
            self.ctxs[self.current].function.chunk.disassemble();
        }
        self.ctxs[0].function.clone()
    }
}
