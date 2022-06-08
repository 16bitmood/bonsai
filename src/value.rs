use crate::common::Chunk;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
}

impl Function {
    pub fn new(arity: usize) -> Function {
        Function {
            arity: arity,
            chunk: Chunk::new(vec![], vec![]),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Float(f64),
    Int(isize),
    Str(String),
    Closure(Function),
    Native(String),
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        match self {
            Value::Bool(x) => *x,
            Value::Int(0) => true,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Bool(x) => write!(f, "{}", x),
            Value::Float(x) => write!(f, "{}", x),
            Value::Int(x) => write!(f, "{}", x),
            Value::Str(x) => write!(f, "{}", x),
            Value::Closure(_) => write!(f, "Closure"),
            Value::Native(x) => write!(f, "Native({})", x),
        }
    }
}
