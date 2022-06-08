use crate::value::Value;
use std::mem;

#[repr(u8)]
pub enum Op {
    // 1-byte Instructions
    Return,
    Pop,
    LoadTrue,

    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,

    // 2-byte Instructions
    LoadConstant,
    SetGlobal,
    GetGlobal,
    SetLocal,
    GetLocal,
    Call,

    // 3-byte Instructions
    RelJump,
    AbsJump,
    JumpIfFalse,
}

impl Op {
    #[inline]
    pub fn from_u8(byte: u8) -> Op {
        unsafe { mem::transmute(byte) }
    }
}

#[derive(Clone, Debug)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new(code: Vec<u8>, constants: Vec<Value>) -> Chunk {
        Chunk {
            code: code,
            constants: constants,
        }
    }

    #[inline]
    pub fn read_byte_double(&self, i: usize) -> usize {
        (self.code[i] as usize) << 8 | (self.code[i + 1] as usize)
    }

    #[inline]
    pub fn write_byte_double(&mut self, i: usize, b: usize) {
        self.code[i] = ((b >> 8) & 0xff) as u8;
        self.code[i + 1] = (b & 0xff) as u8;
    }

    pub fn add_constant(&mut self, constant: Value) -> usize {
        self.constants.push(constant);
        self.constants.len() - 1
    }

    fn disassemble_at(&self, i: usize) -> (String, usize) {
        match Op::from_u8(self.code[i]) {
            // 1-byte Instructions
            Op::Return => ("return".to_string(), 1),
            Op::Pop => ("pop".to_string(), 1),
            Op::LoadTrue => ("load_true".to_string(), 1),

            Op::Negate => ("negate".to_string(), 1),
            Op::Add => ("add".to_string(), 1),
            Op::Subtract => ("subtract".to_string(), 1),
            Op::Multiply => ("multiply".to_string(), 1),
            Op::Divide => ("divide".to_string(), 1),

            // 2-byte Instructions
            Op::LoadConstant => {
                let idx = self.code[i + 1];
                let val = &self.constants[idx as usize];
                (format!("load_constant {:#04x} ({})", idx, val), 2)
            }

            Op::SetGlobal => {
                let name = &self.constants[self.code[i + 1] as usize];
                (format!("set_global {}", name), 2)
            }

            Op::GetGlobal => {
                let name = &self.constants[self.code[i + 1] as usize];
                (format!("get_global {}", name), 2)
            }

            Op::SetLocal => {
                let idx = self.code[i + 1];
                (format!("set_local {:#04x}", idx), 2)
            }

            Op::GetLocal => {
                let idx = self.code[i + 1];
                (format!("get_local {:#04x}", idx), 2)
            }

            Op::Call => {
                let n_args = self.code[i + 1];
                (format!("call {:#04x}", n_args), 2)
            }

            // 3-byte Instructions
            Op::RelJump => {
                let offset = self.read_byte_double(i + 1);
                (format!("rel_jump {:#04x}", offset), 3)
            }

            Op::AbsJump => {
                let offset = self.read_byte_double(i + 1);
                (format!("abs_jump {:#04x}", offset), 3)
            }

            Op::JumpIfFalse => {
                let offset = self.read_byte_double(i + 1);
                (format!("jump_if_false {:#04x}", offset), 3)
            }
        }
    }

    pub fn disassemble(&self) {
        let mut i = 0;
        println!("Constants: {:?}", self.constants);
        while i < self.code.len() {
            let (s, j) = self.disassemble_at(i);
            println!("| {:#04x} : {}", i, s);
            i += j;
        }
    }
}

#[derive(Clone, Debug)]
pub enum Core {
    // Literal
    Lit(Value),

    // Higher Values
    Lambda(Vec<String>, Box<Core>),

    // Variable
    Let(String, Box<Core>), // Variable Declaraction
    Set(String, Box<Core>), // Variable Mutation
    Get(String),            // Variable Access

    // Control Flow
    If(Box<Core>, Box<Core>, Box<Core>),
    Loop(Box<Core>),
    Continue,
    Break,

    // Scope
    Block(Vec<Core>),

    // Function Application
    Call(Box<Core>, Vec<Core>),
    Return(Box<Core>),
}
