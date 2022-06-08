use std::collections::HashMap;

use crate::value::Value;

type NativeFn = Box<dyn Fn(&Value) -> Value>;

pub struct FFI {
    map: HashMap<String, NativeFn>,
}

impl FFI {
    pub fn new() -> FFI {
        FFI {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, s: String, f: NativeFn) {
        self.map.insert(s, f);
    }

    pub fn call(&self, s: &String, arg: &Value) -> Value {
        self.map.get(s).unwrap()(arg)
    }

    pub fn has(&self, s: &String) -> bool {
        self.map.contains_key(s)
    }
}
