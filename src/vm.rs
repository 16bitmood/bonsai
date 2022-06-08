use std::collections::HashMap;

use crate::common::Op;
use crate::native::FFI;
use crate::value::{Function, Value};

#[derive(Clone)]
pub struct CallFrame {
    ip: usize,
    function: Function,
    stack_start: usize,
}

impl CallFrame {
    pub fn new(f: Function, off: usize) -> CallFrame {
        CallFrame {
            ip: 0,
            function: f,
            stack_start: off,
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
    // native_functions: HashMap<String, Box<T>>,
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
}

impl VM<'_> {
    pub fn new(f: Function, natives: &FFI) -> VM {
        // TODO: Make call-stack static.
        let initial_frame: CallFrame = CallFrame::new(f, 0);
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
        self.frames[self.current_frame].function.chunk.code[ip]
    }

    #[inline]
    fn read_byte_double(&self, ip: usize) -> usize {
        self.frames[self.current_frame]
            .function
            .chunk
            .read_byte_double(ip)
    }

    #[inline]
    fn get_constant(&self, idx: usize) -> Value {
        self.frames[self.current_frame].function.chunk.constants[idx].clone()
    }

    pub fn run(&mut self) -> VMResult {
        loop {
            let ip = self.get_ip();
            match Op::from_u8(self.read_byte(ip)) {
                // 1-byte Instructions
                Op::Return => {
                    let result = self.stack.pop().unwrap();
                    self.frames.pop();
                    if self.frames.len() == 0 {
                        return VMResult::Ok;
                    }

                    self.current_frame -= 1;
                    self.stack.push(result);
                }

                Op::Pop => {
                    self.stack.pop();
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
                    self.stack[ss + idx] = self.stack.pop().unwrap();
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
                    let top = self.stack.pop().unwrap();

                    self.offset_ip(2);

                    println!("Calling {} with stack: {:?}", top, self.stack);

                    match top {
                        Value::Closure(f) => {
                            self.current_frame += 1;
                            self.frames
                                .push(CallFrame::new(f, self.stack.len() - nargs));
                        }

                        Value::Native(name) => {
                            let result = self.ffi.call(&name, &self.stack.pop().unwrap());
                            self.stack.push(result);
                        }
                        _ => {
                            // println!("top: {:?}", top);
                            // println!("{:?}", self.stack);
                            // println!("{:?}", self.get_ip());
                            todo!("runtime error");
                        }
                    }
                }

                // 3-byte Instructions
                Op::JumpIfFalse => {
                    let offset = self.read_byte_double(ip + 1);
                    if self.stack.last().unwrap().is_falsey() {
                        self.offset_ip(offset);
                    } else {
                        self.offset_ip(3);
                    }
                }

                Op::RelJump => {
                    let offset = self.read_byte_double(ip + 1);
                    self.offset_ip(offset);
                }

                Op::AbsJump => {
                    let offset = self.read_byte_double(ip + 1);
                    self.set_ip(offset);
                }
            }
        }
    }
}
