use core::ops::{AddAssign, Not};
#[derive(Clone, Debug, PartialEq)]
pub enum Object {
    String(String),
    Number(f64),
    Bool(bool),
    List(Vec<Object>),
    Null,
    Invalid,
}

impl Object {
    pub fn to_string(&self) -> Object {
        match self {
            Object::String(_) => self.clone(),
            Object::Number(num) => Object::String(num.to_string()),
            Object::Bool(b) => Object::String(b.to_string()),
            Object::Null => Object::String("".to_string()),
            _ => Object::String(String::new()),
        }
    }
    pub fn to_number(&self) -> Object {
        match self {
            Object::String(string) => string
                .parse::<f64>()
                .map_or(Object::Invalid, Object::Number),
            Object::Number(_) => self.clone(),
            Object::Bool(b) => Object::Number(if *b { 1.0 } else { 0.0 }),
            Object::Null => Object::Number(0.0),
            _ => Object::Number(0.0),
        }
    }
    pub fn to_bool(&self) -> Object {
        match self {
            Object::String(string) => Object::Bool(!string.is_empty()),
            Object::Number(num) => Object::Bool(if *num != 0.0 { true } else { false }),
            Object::Bool(_) => self.clone(),
            Object::Null => Object::Bool(false),
            _ => Object::Bool(false),
        }
    }
    pub fn convert_to(&mut self, other: Self) {
        match other {
            Object::String(_) => *self = self.to_string(),
            Object::Number(_) => *self = self.to_number(),
            Object::Bool(_) => *self = self.to_bool(),
            _ => {}
        }
    }
    pub fn set_to(&mut self, value: Self) {
        *self = value;
    }
    pub fn get_string_value(&self) -> String {
        let tmp = self.to_string();
        match tmp {
            Object::String(s) => s,
            _ => String::new(),
        }
    }
    pub fn get_number_value(&self) -> f64 {
        let tmp = self.to_number();
        match tmp {
            Object::Number(n) => n,
            _ => 0.0,
        }
    }
    pub fn get_bool_value(&self) -> bool {
        let tmp = self.to_bool();
        match tmp {
            Object::Bool(b) => b,
            _ => false,
        }
    }
    pub fn get_list_index(&self, i: usize) -> Object {
        match self {
            Object::List(list) => list.get(i).unwrap_or(&Object::Null).clone(),
            Object::String(s) => match s.chars().nth(i) {
                Some(c) => Object::String(c.to_string()),
                None => Object::Null,
            },
            _ => Object::Null,
        }
    }
}

impl Not for Object {
    type Output = bool;
    fn not(self) -> <Self as Not>::Output {
        match self {
            Object::Number(num) => num == 0.0,
            Object::Bool(b) => !b,
            Object::String(string) => string.is_empty(),
            _ => false,
        }
    }
}

impl AddAssign for Object {
    fn add_assign(&mut self, rhs: Self) {
        match self {
            Object::Number(num) => {
                if let Object::Number(n) = rhs {
                    *num += n;
                } else if let Object::String(s) = rhs {
                    num.to_string().push_str(&s);
                }
            }
            _ => (),
        }
    }
}
