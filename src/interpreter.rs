use crate::{lexer::TokenType, object::Object, parser::Tree};
use core::iter::Iterator;
use std::{collections::HashMap, usize};

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
    //TODO move this to Object Struct
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
                (Object::List(ref list), Object::Number(num)) => {
                    let mut new_list = vec![];
                    for _ in 0..num as usize {
                        for item in list {
                            new_list.push(item.clone());
                        }
                    }
                    Object::List(new_list)
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
            TokenType::BitAnd => left & right,
            TokenType::BitOR => left | right,
            TokenType::Shl => left << right,
            TokenType::Shr => left >> right,
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
            TokenType::And => Object::Bool(
                left.to_bool_obj().get_bool_value() && right.to_bool_obj().get_bool_value(),
            ),
            TokenType::Or => Object::Bool(
                left.to_bool_obj().get_bool_value() || right.to_bool_obj().get_bool_value(),
            ),
            _ => Object::Bool(false),
        }
    }

    pub fn interpret(&mut self, tree: Tree) -> Object {
        match tree {
            Tree::Empty() => Object::Null,
            Tree::Number(num) => Object::Number(num),
            Tree::Bool(b) => Object::Bool(b),
            Tree::String(string) => Object::String(string),
            Tree::List(list) => {
                let mut buf = vec![];
                list.iter().for_each(|item| {
                    buf.push(self.interpret(item.clone()));
                });
                Object::List(buf)
            }
            Tree::Ident(var) => self.get_var(var).unwrap_or(&mut Object::Null).clone(),
            Tree::Range(start, end) => {
                let start_obj = self.interpret(*start);
                let end_obj = self.interpret(*end);
                if let (Object::Number(s), Object::Number(e)) = (start_obj, end_obj) {
                    return Object::Range(s, e);
                }
                Object::Invalid
            }
            Tree::ListCall(var, index) => self
                .interpret(*var)
                .get_list_index(self.interpret(*index).to_number_obj().get_number_value() as usize),
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
                let value_obj = self.interpret(*value.clone());
                match *var {
                    Tree::Ident(name) => {
                        for scope in self.scopes.iter_mut().rev() {
                            if scope.contains_key(&name) {
                                scope.insert(name, value_obj.clone());
                                return value_obj;
                            }
                        }
                    }
                    Tree::ListCall(var, index) => {
                        let index_num =
                            self.interpret(*index).to_number_obj().get_number_value() as usize;

                        if let Some(var_obj) = self.interpret_mut(*var) {
                            var_obj.set_list_index(index_num, value_obj);
                            return var_obj.clone();
                        }
                    }
                    _ => {}
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
                if expr_obj.to_bool_obj().get_bool_value() {
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
                                if expr_stmt_obj.to_bool_obj().get_bool_value() {
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
                self.enter_scope();
                while expr_obj.to_bool_obj().get_bool_value() {
                    self.body_block(&body);
                    expr_obj = self.interpret(*expr.clone());
                }
                self.exit_scope();
                Object::Null
            }

            Tree::For {
                var,
                expr,
                mut body,
            } => {
                let expr_obj = self.interpret(*expr.clone());
                match expr_obj {
                    Object::Range(start, end) => {
                        self.enter_scope();
                        self.set_var(var.clone(), Object::Number(start));
                        let mask = ((((start < end) as i32) << 1) - 1) as f64;
                        body.push(Tree::Assign(
                            Box::new(Tree::Ident(var.clone())),
                            Box::new(Tree::BinOp(
                                Box::new(Tree::Ident(var.clone())),
                                TokenType::Plus,
                                Box::new(Tree::Number(mask)),
                            )),
                        ));
                        while self.get_var(var.clone()).unwrap().get_number_value() != end {
                            self.body_block(&body);
                        }
                        self.exit_scope();
                    }
                    Object::String(string) => {
                        self.enter_scope();
                        for c in string.chars() {
                            let var_obj = self.set_var(var.clone(), Object::String(String::new()));
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
                        self.exit_scope();
                    }
                    Object::List(list) => {
                        self.enter_scope();
                        for item in list {
                            self.set_var(var.clone(), item);
                            self.body_block(&body);
                        }
                        self.exit_scope();
                    }
                    _ => (),
                };
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

            Tree::Dbg(expr) => {
                let expr_obj = self.interpret(*expr);
                println!("{expr_obj}");
                Object::Null
            }

            _ => Object::Null,
        }
    }

    // A Helper Method to mut Objects
    fn interpret_mut(&mut self, tree: Tree) -> Option<&mut Object> {
        match tree {
            Tree::Ident(name) => self.get_var(name),
            Tree::ListCall(list, index) => {
                let index_num = self.interpret(*index).to_number_obj().get_number_value() as usize;
                self.interpret_mut(*list)
                    .unwrap()
                    .get_list_index_mut(index_num)
            }
            _ => None,
        }
    }

    fn body_block(&mut self, body: &Vec<Tree>) {
        body.iter().for_each(|stmt| {
            self.interpret(stmt.clone());
        });
    }
}
