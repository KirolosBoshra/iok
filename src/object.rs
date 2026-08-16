use crate::file_handler::FileHandler;
use crate::interner::{intern, resolve};
use crate::lexer::Loc;
use crate::logger::{ErrorType, Logger};
use crate::parser::Node;
use crate::socket::Socket;
use crate::std_native::NativeFn;
use core::ops::{AddAssign, BitAnd, Not, Shl, Shr};
use lazy_static::lazy_static;
use rustc_hash::FxHashMap;
use std::{fmt, ops::BitOr, rc::Rc};

lazy_static! {
    pub static ref M_LEN: u32 = intern("len");
    pub static ref M_PUSH: u32 = intern("push");
    pub static ref M_POP: u32 = intern("pop");
    pub static ref M_INCLUDES: u32 = intern("includes");
    pub static ref M_JOIN: u32 = intern("join");
    pub static ref M_SUBSTR: u32 = intern("substr");
    pub static ref M_SPLIT: u32 = intern("split");
    pub static ref M_ORD: u32 = intern("ord");
    pub static ref M_TRIM: u32 = intern("trim");
    pub static ref M_UPPER: u32 = intern("to_upper");
    pub static ref M_LOWER: u32 = intern("to_lower");
    pub static ref M_TO_NUMBER: u32 = intern("to_number");
    pub static ref M_REPLACE: u32 = intern("replace");
}

#[derive(Clone, Debug)]
pub enum Object {
    String(Rc<String>),
    Number(f64),
    Bool(bool),
    List(Vec<Object>),
    Range(f64, f64),
    Ret(Box<Object>),
    File(FileHandler),
    Socket(Socket),
    Fn {
        name: u32,
        args: Vec<(u32, Object)>,
        body: Rc<Vec<Node>>,
    },
    NativeFn {
        name: String,
        function: NativeFn,
    },
    StructDef {
        name: u32,
        fields: Rc<FxHashMap<u32, Object>>,
        methods: Rc<FxHashMap<u32, Object>>,
    },
    Instance {
        struct_def: Box<Object>,
        fields: Rc<FxHashMap<u32, Object>>,
    },
    NameSpace {
        name: u32,
        namespace: Box<FxHashMap<u32, Object>>,
    },
    Null,
    Invalid,
    Break,
    Continue,
}

impl Object {
    pub fn to_string_obj(&self) -> Object {
        match self {
            Object::String(ref s) => Object::String(Rc::clone(s)),
            Object::Number(num) => Object::String(Rc::new(num.to_string())),
            Object::Bool(b) => Object::String(Rc::new(b.to_string())),
            Object::Null => Object::String(Rc::new(String::new())),
            _ => Object::String(Rc::new(String::new())),
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
            (*s).clone()
        } else {
            String::new()
        }
    }

    #[inline]
    pub fn to_f64(&self) -> f64 {
        match self {
            Object::String(s) => s.parse().map_or(0.0, |n: f64| n),
            Object::Number(n) => *n,
            Object::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    #[inline]
    pub fn to_bool(&self) -> bool {
        match self {
            Object::String(s) => !s.is_empty(),
            Object::Number(num) => *num != 0.0,
            Object::Bool(b) => *b,
            _ => false,
        }
    }

    pub fn get_bool_value(&self) -> bool {
        if let Object::Bool(b) = self {
            *b
        } else {
            false
        }
    }

    #[inline]
    pub fn get_list_index(&self, i: usize) -> Object {
        match self {
            Object::List(list) => list.get(i).cloned().unwrap_or(Object::Null),
            Object::String(s) => s.as_bytes().get(i).map_or(Object::Null, |&b| {
                Object::String(Rc::new((b as char).to_string()))
            }),
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

    pub fn get_field_mut(&mut self, name: &u32) -> Option<&mut Object> {
        match self {
            Object::Instance {
                struct_def: _,
                ref mut fields,
            } => Rc::make_mut(fields).get_mut(name),
            Object::NameSpace { namespace, .. } => namespace.get_mut(name),
            _ => None,
        }
    }

    pub fn get_field(&self, name: &u32) -> Option<&Object> {
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
                let s = Rc::make_mut(s);
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

    pub fn simple_method(&mut self, name: u32, args: &[Object], loc: Loc) -> Object {
        match name {
            id if id == *M_LEN => Object::Number(self.get_len() as f64),
            id if id == *M_PUSH => {
                if args.len() != 1 {
                    Logger::error(
                        &format!("Expected 1 arg found {}", args.len()),
                        Some(loc),
                        ErrorType::RunTime,
                    );
                    return Object::Null;
                }
                self.push(args[0].clone());
                Object::Null
            }
            id if id == *M_POP => self.pop(),
            id if id == *M_INCLUDES => {
                if let Some(Object::String(search)) = args.first() {
                    if let Object::String(s) = self {
                        return Object::Bool(s.contains(search.as_str()));
                    }
                }
                Object::Null
            }
            id if id == *M_JOIN => {
                if let Some(Object::String(sep)) = args.first() {
                    if let Object::List(items) = self {
                        let joined = items
                            .iter()
                            .map(|o| o.to_string())
                            .collect::<Vec<_>>()
                            .join(sep.as_str());
                        return Object::String(Rc::new(joined));
                    }
                }
                Object::Null
            }
            id if id == *M_SUBSTR => {
                if let (Some(Object::Number(start)), Some(Object::Number(len))) =
                    (args.first(), args.get(1))
                {
                    if let Object::String(s) = self {
                        let bytes = s.as_bytes();
                        let to = *start as usize + *len as usize;
                        let slice = bytes.get(*start as usize..to).unwrap_or(&[]);
                        return Object::String(Rc::new(
                            String::from_utf8_lossy(slice).into_owned(),
                        ));
                    }
                }
                Object::Null
            }
            id if id == *M_SPLIT => {
                if let Some(Object::String(sep)) = args.first() {
                    if let Object::String(s) = self {
                        return Object::List(
                            s.split(sep.as_str())
                                .map(|p| Object::String(Rc::new(p.to_string())))
                                .collect(),
                        );
                    }
                }
                Object::Null
            }
            id if id == *M_ORD => {
                if let Some(Object::Number(i)) = args.first() {
                    if let Object::String(s) = self {
                        return s
                            .as_bytes()
                            .get(*i as usize)
                            .map_or(Object::Null, |&b| Object::Number(b as f64));
                    }
                }
                Object::Null
            }
            id if id == *M_TRIM => {
                if let Object::String(s) = self {
                    return Object::String(Rc::new(s.trim().to_string()));
                }
                Object::Null
            }
            id if id == *M_UPPER => {
                if let Object::String(s) = self {
                    return Object::String(Rc::new(s.to_uppercase()));
                }
                Object::Null
            }
            id if id == *M_LOWER => {
                if let Object::String(s) = self {
                    return Object::String(Rc::new(s.to_lowercase()));
                }
                Object::Null
            }
            id if id == *M_TO_NUMBER => {
                if let Object::String(s) = self {
                    return s.parse::<f64>().map_or(Object::Null, Object::Number);
                }
                Object::Null
            }
            id if id == *M_REPLACE => {
                if let (Some(Object::String(from)), Some(Object::String(to))) =
                    (args.first(), args.get(1))
                {
                    if let Object::String(s) = self {
                        return Object::String(Rc::new(s.replace(from.as_str(), to.as_str())));
                    }
                }
                Object::Null
            }
            _ => Object::Null,
        }
    }

    pub fn push(&mut self, obj: Object) {
        match self {
            Object::List(ref mut list) => {
                list.push(obj);
            }
            Object::String(ref mut s) => Rc::make_mut(s).push_str(&obj.to_string()),
            _ => {}
        }
    }

    pub fn pop(&mut self) -> Object {
        match self {
            Object::List(ref mut list) => list.pop().expect("List is empty"),
            Object::String(ref mut s) => Object::String(Rc::new(
                Rc::make_mut(s).pop().expect("String is Empty").to_string(),
            )),
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
            (Object::Break, Object::Break) => true,
            (Object::Continue, Object::Continue) => true,
            _ => false,
        }
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Object::String(s) => write!(f, "\"{s}\""),
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
            } => {
                let args_str: Vec<String> = args
                    .iter()
                    .map(|(id, default)| match default {
                        Object::Null => resolve(*id),
                        d => format!("{} = {}", resolve(*id), d),
                    })
                    .collect();
                write!(f, "fn {}({})", resolve(*name), args_str.join(", "))
            }
            Object::NativeFn { name, .. } => write!(f, "NativeFn<{name}>"),
            Object::StructDef {
                name,
                fields: _,
                methods: _,
            } => write!(f, "<{}>", resolve(*name)),
            Object::Instance {
                struct_def: def,
                fields: _,
            } => write!(f, "Object{def}"),
            Object::NameSpace { name, .. } => write!(f, "@{}", resolve(*name)),
            Object::Null => write!(f, "null"),
            Object::Invalid => write!(f, "invalid"),
            Object::Break => write!(f, "break"),
            Object::Continue => write!(f, "continue"),
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
                let s_mut = Rc::make_mut(s);
                match rhs {
                    Object::String(r) => s_mut.push_str(&r),
                    other => s_mut.push_str(&other.to_string_obj().get_string_value()),
                }
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
