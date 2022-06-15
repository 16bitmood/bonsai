use crate::common::Chunk;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Function {
    pub arity: usize,
    pub upvalue_count: usize,
    pub chunk: Chunk,
}

pub type HeapedData = Rc<RefCell<Value>>;

impl Function {
    pub fn new(arity: usize, upvalue_count: usize, chunk: Chunk) -> Function {
        Function {
            arity,
            upvalue_count,
            chunk,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub function: Function,
    pub upvalues: Rc<RefCell<Vec<HeapedData>>>,
}

impl Closure {
    pub fn new(function: Function) -> Closure {
        Closure {
            function,
            upvalues: Rc::new(RefCell::new(vec![])),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    None,
    Bool(bool),
    Float(f64),
    Int(isize),
    Str(String),
    Closure(Closure),
    Function(Function),
    HeapedData(HeapedData),
    Native(String),
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        match self {
            Value::Bool(x) => !x,
            Value::Int(0) => true,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::None => write!(f, "None"),
            Value::Bool(x) => write!(f, "{}", x),
            Value::Float(x) => write!(f, "{}", x),
            Value::Int(x) => write!(f, "{}", x),
            Value::Str(x) => write!(f, "{}", x),
            Value::Closure(_) => write!(f, "Closure"),
            Value::Function(_) => write!(f, "Function"),
            Value::HeapedData(x) => write!(f, "{}", x.borrow()),
            Value::Native(x) => write!(f, "Native({})", x),
        }
    }
}
