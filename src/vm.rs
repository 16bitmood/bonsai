use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::common::Op;
use crate::native::FFI;
use crate::value::{Closure, HeapedData, Value};

#[derive(Clone)]
pub struct CallFrame {
    ip: usize,
    closure: Closure,
    stack_start: usize,
}

impl CallFrame {
    pub fn new(closure: Closure, stack_start: usize) -> CallFrame {
        CallFrame {
            ip: 0,
            closure,
            stack_start,
        }
    }
}

pub enum VMResult {
    Ok,
    Error,
}

pub struct VM<'a> {
    frames: Vec<CallFrame>,
    current_frame: usize,
    ffi: &'a FFI,
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
}

impl VM<'_> {
    pub fn new(c: Closure, natives: &FFI) -> VM {
        // TODO: Make call-stack static.
        let initial_frame: CallFrame = CallFrame::new(c, 0);
        let vm = VM {
            frames: vec![initial_frame],
            ffi: natives,
            current_frame: 0,
            stack: vec![],
            globals: HashMap::new(),
        };
        vm
    }

    #[inline]
    fn stack_start(&self) -> usize {
        self.frames[self.current_frame].stack_start
    }

    #[inline]
    fn get_ip(&self) -> usize {
        self.frames[self.current_frame].ip
    }

    #[inline]
    fn offset_ip(&mut self, by: usize) {
        self.frames[self.current_frame].ip += by;
    }

    #[inline]
    fn set_ip(&mut self, ip: usize) {
        self.frames[self.current_frame].ip = ip;
    }

    #[inline]
    fn read_byte(&self, ip: usize) -> u8 {
        self.frames[self.current_frame].closure.function.chunk.code[ip]
    }

    #[inline]
    fn read_byte_double(&self, ip: usize) -> usize {
        self.frames[self.current_frame]
            .closure
            .function
            .chunk
            .read_byte_double(ip)
    }

    #[inline]
    fn get_constant(&self, idx: usize) -> Value {
        self.frames[self.current_frame]
            .closure
            .function
            .chunk
            .constants[idx]
            .clone()
    }

    fn capture_upvalue(&mut self, idx: usize) -> HeapedData {
        let val = self.stack[idx].clone();
        match &self.stack[idx] {
            Value::Closure(c) => Rc::new(RefCell::new(Value::Closure(Closure {
                function: c.function.clone(),
                upvalues: Rc::clone(&c.upvalues),
            }))),
            Value::HeapedData(x) => Rc::clone(&x),
            _ => {
                let val_ref = Rc::new(RefCell::new(val));
                Rc::clone(&val_ref)
            }
        }
    }

    pub fn run(&mut self, dbg: bool) -> VMResult {
        while self.get_ip()
            < self.frames[self.current_frame]
                .closure
                .function
                .chunk
                .code
                .len()
        {
            let ip = self.get_ip();
            if dbg { // Debug Info
                println!("-");
                print!("Stack {}: [ ", self.frames[self.current_frame].stack_start);
                for (i, x) in self.stack.iter().enumerate() {
                    print!("{}", x);
                    if i != self.stack.len() - 1 {
                        print!(", ");
                    }
                }
                println!(" ]");
                println!(
                    "{}",
                    self.frames[self.current_frame]
                        .closure
                        .function
                        .chunk
                        .disassemble_at(ip)
                        .0
                );
            }

            match Op::from_u8(self.read_byte(ip)) {
                // 1-byte Instructions
                Op::Return => {
                    let result = self.stack.pop().unwrap();
                    let drain_from = self.frames.pop().unwrap().stack_start;
                    if self.frames.len() == 0 {
                        return VMResult::Ok;
                    }
                    self.stack.drain(drain_from..self.stack.len());

                    self.current_frame -= 1;
                    self.stack.push(result);
                }

                Op::Pop => {
                    self.stack.pop();
                    self.offset_ip(1);
                }

                Op::LoadTrue => {
                    self.stack.push(Value::Bool(true));
                    self.offset_ip(1);
                }

                Op::Negate => {
                    let x = self.stack.pop().unwrap();
                    match x {
                        Value::Bool(x) => self.stack.push(Value::Bool(!x)),
                        Value::Float(x) => self.stack.push(Value::Float(-x)),
                        Value::Int(x) => self.stack.push(Value::Int(-x)),
                        _ => todo!("runtime error"),
                    }
                    self.offset_ip(1);
                }

                Op::IsEqual => {
                    let x = self.stack.pop().unwrap();
                    let y = self.stack.pop().unwrap();
                    match (x, y) {
                        (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Bool(x == y)),
                        (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Bool(x == y)),
                        (Value::Bool(x), Value::Bool(y)) => self.stack.push(Value::Bool(x == y)),
                        (_, _) => self.stack.push(Value::Bool(false)),
                    }
                    self.offset_ip(1);
                }

                Op::Add => {
                    let y = self.stack.pop().unwrap();
                    let x = self.stack.pop().unwrap();
                    match (x, y) {
                        (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x + y)),
                        (Value::Float(x), Value::Int(y)) => {
                            self.stack.push(Value::Float(x + y as f64))
                        }
                        (Value::Int(x), Value::Float(y)) => {
                            self.stack.push(Value::Float(x as f64 + y))
                        }
                        (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x + y)),
                        _ => todo!("runtime error"),
                    }
                    self.offset_ip(1);
                }

                Op::Subtract => {
                    let y = self.stack.pop().unwrap();
                    let x = self.stack.pop().unwrap();
                    match (x, y) {
                        (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x - y)),
                        (Value::Float(x), Value::Int(y)) => {
                            self.stack.push(Value::Float(x - y as f64))
                        }
                        (Value::Int(x), Value::Float(y)) => {
                            self.stack.push(Value::Float(x as f64 - y))
                        }
                        (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x - y)),
                        _ => todo!("runtime error"),
                    }
                    self.offset_ip(1);
                }

                Op::Multiply => {
                    let y = self.stack.pop().unwrap();
                    let x = self.stack.pop().unwrap();
                    match (x, y) {
                        (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x * y)),
                        (Value::Float(x), Value::Int(y)) => {
                            self.stack.push(Value::Float(x * y as f64))
                        }
                        (Value::Int(x), Value::Float(y)) => {
                            self.stack.push(Value::Float(x as f64 * y))
                        }
                        (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x * y)),
                        _ => todo!("runtime error"),
                    }
                    self.offset_ip(1);
                }

                Op::Divide => {
                    let y = self.stack.pop().unwrap();
                    let x = self.stack.pop().unwrap();
                    match (x, y) {
                        (Value::Int(x), Value::Int(y)) => {
                            self.stack.push(Value::Float(x as f64 / y as f64))
                        }
                        (Value::Float(x), Value::Int(y)) => {
                            self.stack.push(Value::Float(x / y as f64))
                        }
                        (Value::Int(x), Value::Float(y)) => {
                            self.stack.push(Value::Float(x as f64 / y))
                        }
                        (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x / y)),
                        _ => todo!("runtime error"),
                    }
                    self.offset_ip(1);
                }

                Op::SetUpvalue => {
                    let idx = self.read_byte(ip + 1) as usize;
                    self.offset_ip(2);
                    let upvalues = self.frames[self.current_frame].closure.upvalues.borrow();
                    let mut up_ref = upvalues[idx].borrow_mut();
                    *up_ref = self.stack.last().unwrap().clone();
                }

                Op::GetUpvalue => {
                    let idx = self.read_byte(ip + 1) as usize;
                    self.stack.push(Value::HeapedData(Rc::clone(
                        &self.frames[self.current_frame].closure.upvalues.borrow()[idx],
                    )));
                    self.offset_ip(2);
                }

                // 2-byte Instructions
                Op::LoadConstant => {
                    let idx = self.read_byte(ip + 1);
                    let x = self.get_constant(idx as usize);
                    self.stack.push(x);
                    self.offset_ip(2);
                }

                Op::SetGlobal => {
                    let name = &self.get_constant(self.read_byte(ip + 1) as usize);
                    let val = self.stack.pop().unwrap();
                    if let Value::Str(x) = name {
                        self.globals.insert(x.clone(), val);
                    } else {
                        todo!("Invalid Set");
                    }
                    self.offset_ip(2);
                }

                Op::GetGlobal => {
                    let name = &self.get_constant(self.read_byte(ip + 1) as usize);
                    if let Value::Str(x) = name {
                        if self.ffi.has(x) {
                            self.stack.push(Value::Native(x.clone()));
                        } else if self.globals.contains_key(x) {
                            let val = self.globals.get(x).unwrap();
                            self.stack.push(val.clone());
                        }
                    } else {
                        todo!("Invalid Get");
                    }

                    self.offset_ip(2);
                }

                Op::SetLocal => {
                    let idx = self.read_byte(ip + 1) as usize;
                    let ss = self.stack_start();
                    self.stack[ss + idx] = self.stack.pop().unwrap().clone();
                    self.offset_ip(2);
                }

                Op::GetLocal => {
                    let idx = self.read_byte(ip + 1) as usize;
                    self.stack
                        .push(self.stack[self.stack_start() + idx].clone());
                    self.offset_ip(2);
                }

                Op::Call => {
                    let nargs = self.read_byte(ip + 1) as usize;
                    let mut f = self.stack.pop().unwrap();

                    if let Value::HeapedData(x) = f {
                        f = x.borrow().clone();
                    }

                    self.offset_ip(2);
                    match f {
                        Value::Closure(c) => {
                            self.current_frame += 1;
                            self.frames
                                .push(CallFrame::new(c, self.stack.len() - nargs));

                            // TODO: Check
                        }

                        Value::Native(name) => {
                            let result = self.ffi.call(&name, &self.stack.pop().unwrap());
                            self.stack.push(result);
                        }

                        _ => {
                            todo!("runtime error");
                        }
                    }
                }

                // 3-byte Instructions
                Op::JumpIfFalse => {
                    let offset = self.read_byte_double(ip + 1);
                    if self.stack.pop().unwrap().is_falsey() {
                        self.offset_ip(offset);
                    } else {
                        self.offset_ip(3);
                    }
                }

                Op::Jump => {
                    let offset = self.read_byte_double(ip + 1);
                    self.offset_ip(offset);
                }

                Op::AbsJump => {
                    let offset = self.read_byte_double(ip + 1);
                    self.set_ip(offset);
                }

                Op::MakeClosure => {
                    let idx = self.read_byte(ip + 1);
                    if let Value::Function(f) = self.get_constant(idx as usize) {
                        let upvalue_count = f.upvalue_count;
                        let closure = Closure::new(f);
                        let upvalues = Rc::clone(&closure.upvalues);
                        self.stack.push(Value::Closure(closure));
                        self.offset_ip(2);

                        for _ in 0..upvalue_count {
                            let lip = self.get_ip();
                            let is_local = self.read_byte(lip);
                            let idx = self.read_byte(lip + 1) as usize;
                            // TODO: Upvalues are cloned
                            if is_local != 0 {
                                upvalues.borrow_mut().push(self.capture_upvalue(
                                    self.frames[self.current_frame].stack_start + idx,
                                ));
                            } else {
                                upvalues.borrow_mut().push(Rc::clone(
                                    &self.frames[self.current_frame].closure.upvalues.borrow()[idx],
                                ));
                            }
                            self.offset_ip(2);
                        }
                        // self.stack.push(Value::Closure(closure));
                    } else {
                        todo!("Can only make functions into closure")
                    }
                }
            }
        }
        VMResult::Ok
    }
}
