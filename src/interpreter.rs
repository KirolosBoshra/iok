use core::iter::Iterator;
use core::ops::{AddAssign, Not};
use std::{collections::HashMap, usize};

use crate::{lexer::TokenType, parser::Tree};

#[derive(Clone, Debug, PartialEq)]
pub enum Object {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    Invalid,
    // Object(Box<Object>),
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

#[derive(Debug)]
pub struct Interpreter {
    scopes: Vec<HashMap<String, Object>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn set_var(&mut self, name: String, value: Object) -> &mut Object {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.clone(), value);
        }
        self.get_var(name.clone()).unwrap()
    }

    fn get_var(&mut self, name: String) -> Option<&mut Object> {
        self.scopes
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(&name))
    }

    fn bin_op(&self, left: Object, op: TokenType, right: Object) -> Object {
        match op {
            TokenType::Plus => match (left, right) {
                (Object::Number(left_num), Object::Number(right_num)) => {
                    Object::Number(left_num + right_num)
                }
                (Object::Number(left_num), Object::String(right_string)) => {
                    Object::String(left_num.to_string() + &right_string)
                }
                (Object::String(left_string), Object::Number(right_num)) => {
                    Object::String(left_string + &right_num.to_string())
                }
                (Object::String(left_string), Object::String(right_string)) => {
                    Object::String(left_string + &right_string)
                }
                _ => Object::Null,
            },
            TokenType::Minus => match (left, right) {
                (Object::Number(left_num), Object::Number(right_num)) => {
                    Object::Number(left_num - right_num)
                }
                (Object::String(left_string), Object::Number(right_num)) => {
                    if left_string.len() < right_num as usize {
                        return Object::Invalid;
                    }
                    Object::String(
                        left_string[..left_string.len() - right_num as usize].to_string(),
                    )
                }
                (Object::String(left_string), Object::String(right_string)) => {
                    Object::String(left_string.replace(&right_string, ""))
                }
                _ => Object::Null,
            },
            TokenType::Multiply => match (left, right) {
                (Object::Number(left_num), Object::Number(right_num)) => {
                    Object::Number(left_num * right_num)
                }
                (Object::Number(left_num), Object::String(right_string)) => {
                    Object::String(right_string.repeat(left_num as usize))
                }
                (Object::String(left_string), Object::Number(right_num)) => {
                    Object::String(left_string.repeat(right_num as usize))
                }
                _ => Object::Null,
            },
            TokenType::Divide => match (left, right) {
                (Object::Number(left_num), Object::Number(right_num)) => {
                    if right_num != 0.0 {
                        Object::Number(left_num / right_num)
                    } else {
                        Object::Invalid
                    }
                }
                _ => Object::Null,
            },
            _ => Object::Invalid,
        }
    }

    fn cmp_op(&self, left: Object, op: TokenType, right: Object) -> Object {
        match op {
            TokenType::EquEqu => Object::Bool(left == right),
            TokenType::NotEqu => Object::Bool(left != right),
            TokenType::Greater => match (left, right) {
                (Object::Number(left_num), Object::Number(right_num)) => {
                    Object::Bool(left_num > right_num)
                }
                _ => Object::Bool(false),
            },
            TokenType::GreatEqu => match (left, right) {
                (Object::Number(left_num), Object::Number(right_num)) => {
                    Object::Bool(left_num >= right_num)
                }
                _ => Object::Bool(false),
            },
            TokenType::Less => match (left, right) {
                (Object::Number(left_num), Object::Number(right_num)) => {
                    Object::Bool(left_num < right_num)
                }
                _ => Object::Bool(false),
            },
            TokenType::LessEqu => match (left, right) {
                (Object::Number(left_num), Object::Number(right_num)) => {
                    Object::Bool(left_num <= right_num)
                }
                _ => Object::Bool(false),
            },
            TokenType::Bang => Object::Bool(!left),
            _ => Object::Bool(false),
        }
    }

    pub fn interpret(&mut self, tree: Tree) -> Object {
        match tree {
            Tree::Empty() => Object::Null,
            Tree::Number(num) => Object::Number(num),
            Tree::Bool(b) => Object::Bool(b),
            Tree::String(string) => Object::String(string),
            Tree::Ident(var) => self.get_var(var).unwrap_or(&mut Object::Null).clone(),
            Tree::BinOp(left, op, right) => {
                let left_obj = self.interpret(*left);
                let right_obj = self.interpret(*right);
                self.bin_op(left_obj, op, right_obj)
            }
            Tree::CmpOp(left, op, right) => {
                let left_obj = self.interpret(*left);
                let right_obj = self.interpret(*right);
                self.cmp_op(left_obj, op, right_obj)
            }

            Tree::Let(var, value) => {
                let value_obj = self.interpret(*value);
                self.set_var(var.clone(), value_obj);
                self.get_var(var).unwrap_or(&mut Object::Null).clone()
            }
            Tree::Assign(var, value) => {
                let value_obj = self.interpret(*value);
                for scope in self.scopes.iter_mut().rev() {
                    if scope.contains_key(&var) {
                        scope.insert(var, value_obj.clone());
                        return value_obj;
                    }
                }
                Object::Null
            }

            Tree::If {
                expr,
                body,
                els,
                els_ifs,
            } => {
                let expr_obj = self.interpret(*expr);
                //TODO THIS SO UGLY
                if expr_obj.to_bool().get_bool_value() {
                    self.enter_scope();
                    body.iter().for_each(|stmt| {
                        self.interpret(stmt.clone());
                    });
                    self.exit_scope();
                    expr_obj
                } else {
                    for stmt in els_ifs {
                        match stmt {
                            Tree::ElsIf { expr, body } => {
                                let expr_stmt_obj = self.interpret(*expr);
                                if expr_stmt_obj.to_bool().get_bool_value() {
                                    self.enter_scope();
                                    body.iter().for_each(|tree| {
                                        self.interpret(tree.clone());
                                    });
                                    self.exit_scope();
                                    return expr_stmt_obj;
                                }
                            }
                            _ => (),
                        }
                    }
                    if !els.is_empty() {
                        self.enter_scope();
                        els.iter().for_each(|tree| {
                            self.interpret(tree.clone());
                        });
                        self.exit_scope();
                    }
                    Object::Bool(false)
                }
            }

            Tree::While { expr, body } => {
                let mut expr_obj = self.interpret(*expr.clone());
                while expr_obj.to_bool().get_bool_value() {
                    self.body_block(&body);
                    expr_obj = self.interpret(*expr.clone());
                }
                Object::Null
            }

            Tree::For {
                var,
                expr,
                mut body,
            } => {
                let expr_obj = self.interpret(*expr.clone());
                match expr_obj {
                    Object::String(string) => {
                        for c in string.chars() {
                            let var_obj = if let Some(v) = self.get_var(var.clone()) {
                                v
                            } else {
                                self.set_var(var.clone(), Object::String(String::new()))
                            };
                            match var_obj {
                                Object::String(s) => {
                                    s.clear();
                                    s.push(c);
                                }
                                _ => {
                                    let mut tmp = String::new();
                                    tmp.push(c);
                                    self.set_var(var.clone(), Object::String(tmp));
                                }
                            }
                            self.body_block(&body);
                        }
                    }
                    Object::Number(num) => {
                        let var_obj = if let Some(v) = self.get_var(var.clone()) {
                            v.convert_to(Object::Number(0.0));
                            match v {
                                Object::Number(_) => v,
                                _ => {
                                    v.set_to(Object::Number(0.0));
                                    v
                                }
                            }
                        } else {
                            self.set_var(var.clone(), Object::Number(0.0))
                        };
                        match var_obj {
                            Object::Number(n) => {
                                let mask = ((((*n < num) as i32) << 1) - 1) as f64;
                                body.push(Tree::Assign(
                                    var.clone(),
                                    Box::new(Tree::BinOp(
                                        Box::new(Tree::Ident(var.clone())),
                                        TokenType::Plus,
                                        Box::new(Tree::Number(mask)),
                                    )),
                                ));
                                while self.get_var(var.clone()).unwrap().get_number_value() != num {
                                    self.body_block(&body);
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => (),
                }
                Object::Null
            }

            Tree::Exit(code) => {
                let exit_code = match self.interpret(*code) {
                    Object::Number(num) => num as i32,
                    Object::Null => 0,
                    _ => -1,
                };
                std::process::exit(exit_code);
            }
            _ => Object::Null,
        }
    }

    fn body_block(&mut self, body: &Vec<Tree>) {
        self.enter_scope();
        body.iter().for_each(|stmt| {
            self.interpret(stmt.clone());
        });
        self.exit_scope();
    }
}