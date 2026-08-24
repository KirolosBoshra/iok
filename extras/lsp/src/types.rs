use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum Type {
    Number,
    String,
    Bool,
    List(Option<Box<Type>>),
    Range,
    Null,
    StructInstance(String),
    StructDef(String),
    Module(String),
    File,
    Socket,
    Server,
    Function {
        params: Vec<String>,
        ret: Box<Type>,
    },
    Unknown,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Number => write!(f, "number"),
            Type::String => write!(f, "string"),
            Type::Bool => write!(f, "bool"),
            Type::List(elem) => match elem {
                Some(inner) => write!(f, "list[{}]", inner),
                None => write!(f, "list"),
            },
            Type::Range => write!(f, "range"),
            Type::Null => write!(f, "null"),
            Type::StructInstance(name) => write!(f, "{}", name),
            Type::StructDef(name) => write!(f, "struct {}", name),
            Type::Module(name) => write!(f, "module {}", name),
            Type::File => write!(f, "File"),
            Type::Socket => write!(f, "Socket"),
            Type::Server => write!(f, "Server"),
            Type::Function { params, ret } => {
                write!(f, "fn({}) -> {}", params.join(", "), ret)
            }
            Type::Unknown => write!(f, "unknown"),
        }
    }
}
