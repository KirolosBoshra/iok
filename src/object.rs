use crate::file_handler::FileHandler;
use crate::parser::Tree;
use crate::socket::Socket;
use crate::std_native::NativeFn;
use core::ops::{AddAssign, BitAnd, Not, Shl, Shr};
use rustc_hash::FxHashMap;
use std::{fmt, ops::BitOr, rc::Rc};

#[derive(Clone, Debug)]
pub enum Object {
    String(Box<String>),
    Number(f64),
    Bool(bool),
    List(Vec<Object>),
    Range(f64, f64),
    Ret(Box<Object>),
    File(FileHandler),
    Socket(Socket),
    Fn {
        name: String,
        args: Vec<(String, Object)>,
        body: Rc<Vec<Tree>>,
    },
    NativeFn {
        name: String,
        function: NativeFn,
    },
    StructDef {
        name: Box<String>,
        fields: Rc<FxHashMap<String, Object>>,
        methods: Rc<FxHashMap<String, Object>>,
    },
    Instance {
        struct_def: Box<Object>,
        fields: FxHashMap<String, Object>,
    },
    NameSpace {
        name: String,
        namespace: Box<FxHashMap<String, Object>>,
    },
    Null,
    Invalid,
}

impl Object {
    pub fn to_string_obj(&self) -> Object {
        match self {
            Object::String(ref s) => Object::String(Box::new(s.to_string())),
            Object::Number(num) => Object::String(Box::new(num.to_string())),
            Object::Bool(b) => Object::String(Box::new(b.to_string())),
            Object::Null => Object::String(Box::new(String::new())),
            _ => Object::String(Box::new(String::new())),
        }
    }

    pub fn to_number_obj(&self) -> Object {
        match self {
            Object::String(s) => s.parse().map_or(Object::Invalid, Object::Number),
            Object::Number(n) => Object::Number(*n),
            Object::Bool(b) => Object::Number(if *b { 1.0 } else { 0.0 }),
            Object::Null => Object::Number(0.0),
            _ => Object::Number(0.0),
        }
    }

    pub fn to_bool_obj(&self) -> Object {
        match self {
            Object::String(s) => Object::Bool(!s.is_empty()),
            Object::Number(num) => Object::Bool(*num != 0.0),
            Object::Bool(b) => Object::Bool(*b),
            Object::Null => Object::Bool(false),
            _ => Object::Bool(false),
        }
    }

    pub fn get_string_value(&self) -> String {
        if let Object::String(s) = self.to_string_obj() {
            *s
        } else {
            String::new()
        }
    }

    pub fn get_number_value(&self) -> f64 {
        if let Object::Number(n) = self {
            *n
        } else {
            0.0
        }
    }

    pub fn get_bool_value(&self) -> bool {
        if let Object::Bool(b) = self {
            *b
        } else {
            false
        }
    }

    pub fn get_list_index(&self, i: usize) -> Object {
        match self {
            Object::List(list) => list.get(i).cloned().unwrap_or(Object::Null),
            Object::String(s) => s
                .chars()
                .nth(i)
                .map_or(Object::Null, |c| Object::String(Box::new(c.to_string()))),
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

    pub fn get_field_mut(&mut self, name: &String) -> Option<&mut Object> {
        match self {
            Object::Instance {
                struct_def: _,
                ref mut fields,
            } => fields.get_mut(name),
            Object::NameSpace { namespace, .. } => namespace.get_mut(name),
            _ => None,
        }
    }

    pub fn get_field(&self, name: &String) -> Option<&Object> {
        match self {
            Object::Instance {
                struct_def: _,
                ref fields,
            } => fields.get(name),
            Object::StructDef {
                name: _,
                fields,
                methods: _,
            } => fields.get(name),
            Object::NameSpace { namespace, .. } => namespace.get(name),
            _ => None,
        }
    }

    pub fn set_list_index(&mut self, i: usize, value: Object) {
        match self {
            Object::List(list) => {
                list[i] = value;
            }
            Object::String(s) => {
                if i >= s.len() {
                    let needed = i + 1 - s.len();
                    s.reserve(needed);
                    s.push_str(&" ".repeat(needed)); // extend exactly to index i
                }
                // Replace one character at position i:
                if let Object::String(v) = value {
                    s.replace_range(i..i + 1, &v);
                }
            }
            _ => {}
        }
    }

    pub fn get_len(&self) -> usize {
        match self {
            Object::String(str) => str.len(),
            Object::List(list) => list.len(),
            _ => 0,
        }
    }

    pub fn push(&mut self, obj: Object) {
        match self {
            Object::List(ref mut list) => {
                list.push(obj);
            }
            Object::String(ref mut s) => s.push_str(&obj.to_string()),
            _ => {}
        }
    }

    pub fn pop(&mut self) -> Object {
        match self {
            Object::List(ref mut list) => list.pop().expect("List is empty"),
            Object::String(ref mut s) => {
                Object::String(Box::new(s.pop().expect("String is Empty").to_string()))
            }
            _ => Object::Invalid,
        }
    }
}

// PartialEq derive trait doesn't work well with Function pointers
impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Object::String(a), Object::String(b)) => a == b,
            (Object::Number(a), Object::Number(b)) => a == b,
            (Object::Bool(a), Object::Bool(b)) => a == b,
            (Object::List(a), Object::List(b)) => a == b,
            (Object::Range(a, b), Object::Range(c, d)) => a == c && b == d,
            (Object::Ret(a), Object::Ret(b)) => a == b,
            (Object::File(a), Object::File(b)) => a == b,
            (Object::Socket(a), Object::Socket(b)) => a == b,
            (
                Object::Fn {
                    name: n1,
                    args: a1,
                    body: b1,
                },
                Object::Fn {
                    name: n2,
                    args: a2,
                    body: b2,
                },
            ) => n1 == n2 && a1 == a2 && b1 == b2,
            (Object::NativeFn { name: n1, .. }, Object::NativeFn { name: n2, .. }) => n1 == n2,
            (
                Object::StructDef {
                    name: n1,
                    fields: f1,
                    methods: m1,
                },
                Object::StructDef {
                    name: n2,
                    fields: f2,
                    methods: m2,
                },
            ) => n1 == n2 && f1 == f2 && m1 == m2,
            (
                Object::Instance {
                    struct_def: s1,
                    fields: f1,
                },
                Object::Instance {
                    struct_def: s2,
                    fields: f2,
                },
            ) => s1 == s2 && f1 == f2,
            (
                Object::NameSpace {
                    name: n1,
                    namespace: ns1,
                },
                Object::NameSpace {
                    name: n2,
                    namespace: ns2,
                },
            ) => n1 == n2 && ns1 == ns2,
            (Object::Null, Object::Null) => true,
            (Object::Invalid, Object::Invalid) => true,
            _ => false,
        }
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Object::String(s) => write!(f, "{s}"),
            Object::Number(n) => write!(f, "{n}"),
            Object::Bool(b) => write!(f, "{b}"),
            Object::List(list) => {
                let list_str: Vec<String> = list.iter().map(|obj| obj.to_string()).collect();
                write!(f, "[{}]", list_str.join(", "))
            }
            Object::Range(s, e) => write!(f, "{s}..{e}"),
            Object::Ret(o) => write!(f, "Ret({o})"),
            Object::File(s) => write!(f, "File<{}>", s.path),
            Object::Socket(s) => write!(f, "{s}"),
            Object::Fn {
                name,
                args,
                body: _,
            } => write!(f, "fn {name} ({:?})", args),
            Object::NativeFn { name, .. } => write!(f, "NativeFn<{name}>"),
            Object::StructDef {
                name,
                fields: _,
                methods: _,
            } => write!(f, "<{name}>"),
            Object::Instance {
                struct_def: def,
                fields: _,
            } => write!(f, "Object{def}"),
            Object::NameSpace { name, .. } => write!(f, "@{name}"),
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
                }
            }
            Object::String(s) => {
                s.push_str(&rhs.to_string_obj().get_string_value());
            }
            Object::List(l) => {
                if let Object::List(mut rl) = rhs {
                    l.append(&mut rl)
                } else {
                    l.push(rhs);
                }
            }
            _ => (),
        }
    }
}

impl BitAnd for Object {
    type Output = Object;
    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Object::Number(l), Object::Number(r)) => Object::Number((l as i64 & r as i64) as f64),
            _ => Object::Invalid,
        }
    }
}

impl BitOr for Object {
    type Output = Object;
    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Object::Number(l), Object::Number(r)) => Object::Number((l as i64 | r as i64) as f64),
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

impl Default for Object {
    fn default() -> Self {
        Object::Null
    }
}
