use crate::parser::Tree;
use core::ops::{AddAssign, BitAnd, Not, Shl, Shr};
use std::{fmt, ops::BitOr};
#[derive(Clone, Debug, PartialEq)]
pub enum Object {
    String(String),
    Number(f64),
    Bool(bool),
    List(Vec<Object>),
    Range(f64, f64),
    Ret(Box<Object>),
    Fn {
        name: String,
        args: Vec<(String, Object)>,
        body: Vec<Tree>,
    },
    Null,
    Invalid,
}

impl Object {
    pub fn to_string_obj(&self) -> Object {
        match self {
            Object::String(_) => self.clone(),
            Object::Number(num) => Object::String(num.to_string()),
            Object::Bool(b) => Object::String(b.to_string()),
            Object::Null => Object::String("".to_string()),
            _ => Object::String(String::new()),
        }
    }
    pub fn to_number_obj(&self) -> Object {
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
    pub fn to_bool_obj(&self) -> Object {
        match self {
            Object::String(string) => Object::Bool(!string.is_empty()),
            Object::Number(num) => Object::Bool(if *num != 0.0 { true } else { false }),
            Object::Bool(_) => self.clone(),
            Object::Null => Object::Bool(false),
            _ => Object::Bool(false),
        }
    }
    // pub fn convert_to(&mut self, other: Self) {
    //     match other {
    //         Object::String(_) => *self = self.to_string_obj(),
    //         Object::Number(_) => *self = self.to_number_obj(),
    //         Object::Bool(_) => *self = self.to_bool_obj(),
    //         _ => {}
    //     }
    // }
    pub fn set_to(&mut self, value: Self) {
        *self = value;
    }
    pub fn get_string_value(&self) -> String {
        let tmp = self.to_string_obj();
        match tmp {
            Object::String(s) => s,
            _ => String::new(),
        }
    }
    pub fn get_number_value(&self) -> f64 {
        let tmp = self.to_number_obj();
        match tmp {
            Object::Number(n) => n,
            _ => 0.0,
        }
    }
    pub fn get_bool_value(&self) -> bool {
        let tmp = self.to_bool_obj();
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
    pub fn get_list_index_mut(&mut self, i: usize) -> Option<&mut Object> {
        match self {
            Object::List(ref mut list) => list.get_mut(i),
            Object::String(_) => Some(self),
            _ => None,
        }
    }
    pub fn set_list_index(&mut self, i: usize, value: Object) {
        match self {
            Object::List(list) => list.get_mut(i).unwrap_or(&mut Object::Null).set_to(value),
            Object::String(s) => {
                if i >= s.len() {
                    // IDK Why s.reserve(n) doesn't work
                    let tmp: Vec<char> = std::iter::repeat(' ').take(10).collect();
                    s.extend(tmp);
                }
                s.replace_range(i..i + 1, &value.to_string_obj().get_string_value());
            }
            _ => {}
        }
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Object::String(s) => write!(f, "\"{}\"", s),
            Object::Number(n) => write!(f, "{}", n),
            Object::Bool(b) => write!(f, "{}", b),
            Object::List(list) => {
                let list_str: Vec<String> = list.iter().map(|obj| obj.to_string()).collect();
                write!(f, "[{}]", list_str.join(", "))
            }
            Object::Range(s, e) => write!(f, "{s}..{e}"),
            Object::Ret(o) => write!(f, "Ret({o})"),
            Object::Fn {
                name,
                args,
                body: _,
            } => write!(f, "fn: {name}, {:?}", args),
            Object::Null => write!(f, "null"),
            Object::Invalid => write!(f, "invalid"),
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
                if let Object::Number(n) = rhs.to_number_obj() {
                    *num += n;
                } else if let Object::String(s) = rhs.to_string_obj() {
                    num.to_string().push_str(&s);
                }
            }
            Object::String(s) => {
                s.push_str(&rhs.to_string());
            }
            _ => (),
        }
    }
}

impl BitAnd for Object {
    type Output = Object;
    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Object::Number(l), Object::Number(r)) => Object::Number((l as i32 & r as i32) as f64),
            _ => Object::Invalid,
        }
    }
}

impl BitOr for Object {
    type Output = Object;
    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Object::Number(l), Object::Number(r)) => Object::Number((l as i32 | r as i32) as f64),
            _ => Object::Invalid,
        }
    }
}

impl Shl for Object {
    type Output = Object;
    fn shl(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Object::Number(l), Object::Number(r)) => {
                Object::Number(((l as i32) << (r as i32)) as f64)
            }
            _ => Object::Invalid,
        }
    }
}
impl Shr for Object {
    type Output = Object;
    fn shr(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Object::Number(l), Object::Number(r)) => {
                Object::Number(((l as i32) >> (r as i32)) as f64)
            }
            _ => Object::Invalid,
        }
    }
}
