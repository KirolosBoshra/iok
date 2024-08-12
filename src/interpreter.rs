use crate::{lexer::TokenType, object::Object, parser::Tree};
use core::iter::Iterator;
use std::collections::HashMap;

// TODO IMPORTENT Use Rc<RefCell<Object>> Smart Pointers

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
        self.get_var(name).unwrap()
    }

    fn get_var(&mut self, name: String) -> Option<&mut Object> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(value) = scope.get_mut(&name) {
                return Some(value);
            }
        }
        None
    }

    fn bin_op(&self, left: Object, op: TokenType, right: Object) -> Object {
        use Object::Invalid;
        use Object::List;
        use Object::Null;
        use Object::Number;
        use Object::String;
        use TokenType::*;

        match op {
            Plus => match (left, right) {
                (Number(l), Number(r)) => Number(l + r),
                (Number(l), String(r)) | (String(r), Number(l)) => String(format!("{l}{r}")),
                (String(l), String(r)) => String(l + &r),
                _ => Null,
            },
            Minus => match (left, right) {
                (Number(l), Number(r)) => Number(l - r),
                (String(l), Number(r)) if l.len() >= r as usize => {
                    String(l[..l.len() - r as usize].to_string())
                }
                (String(l), String(r)) => String(l.replace(&r, "")),
                _ => Null,
            },
            Multiply => match (left, right) {
                (Number(l), Number(r)) => Number(l * r),
                (Number(l), String(r)) | (String(r), Number(l)) => String(r.repeat(l as usize)),
                (List(ref l), Number(r)) => List(
                    l.iter()
                        .cycle()
                        .take(l.len() * r as usize)
                        .cloned()
                        .collect(),
                ),
                _ => Null,
            },
            Divide => match (left, right) {
                (Number(l), Number(r)) if r != 0.0 => Number(l / r),
                _ => Invalid,
            },
            BitAnd => left & right,
            BitOR => left | right,
            Shl => left << right,
            Shr => left >> right,
            _ => Invalid,
        }
    }

    fn cmp_op(&self, left: Object, op: TokenType, right: Object) -> Object {
        use Object::{Bool, Number};

        match op {
            // Direct equality and inequality checks
            TokenType::EquEqu => return Bool(left == right),
            TokenType::NotEqu => return Bool(left != right),

            // Lazy evaluation for greater/less comparison
            TokenType::Greater => {
                if let (Number(left_num), Number(right_num)) = (left, right) {
                    return Bool(left_num > right_num);
                }
                Bool(false)
            }
            TokenType::GreatEqu => {
                if let (Number(left_num), Number(right_num)) = (left, right) {
                    return Bool(left_num >= right_num);
                }
                Bool(false)
            }
            TokenType::Less => {
                if let (Number(left_num), Number(right_num)) = (left, right) {
                    return Bool(left_num < right_num);
                }
                Bool(false)
            }
            TokenType::LessEqu => {
                if let (Number(left_num), Number(right_num)) = (left, right) {
                    return Bool(left_num <= right_num);
                }
                Bool(false)
            }

            // Logical NOT, AND, OR operations
            TokenType::Bang => return Bool(!left),
            TokenType::And => {
                // Short-circuit evaluation for `&&` operation
                if left.to_bool_obj().get_bool_value() {
                    return Bool(right.to_bool_obj().get_bool_value());
                } else {
                    return Bool(false);
                }
            }
            TokenType::Or => {
                // Short-circuit evaluation for `||` operation
                if left.to_bool_obj().get_bool_value() {
                    return Bool(true);
                } else {
                    return Bool(right.to_bool_obj().get_bool_value());
                }
            }

            // Default case if none of the above match
            _ => Bool(false),
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
            Tree::Ret(expr) => Object::Ret(Box::new(self.interpret(*expr))),
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
                let v_obj = self.interpret(*value.clone());
                let value_obj = if let Object::Ret(expr) = v_obj {
                    *expr
                } else {
                    v_obj
                };
                self.set_var(var.clone(), value_obj).clone()
            }

            Tree::Assign(var, value) => {
                let mut value_obj = self.interpret(*value);

                if let Object::Ret(expr) = value_obj {
                    value_obj = *expr;
                }

                match *var {
                    Tree::Ident(ref name) => {
                        for scope in self.scopes.iter_mut().rev() {
                            if let Some(existing_value) = scope.get(name) {
                                if matches!(existing_value, Object::Fn { .. }) {
                                    return Object::Invalid;
                                }
                                scope.insert(name.clone(), value_obj.clone());
                            }
                        }
                    }
                    Tree::ListCall(ref var, index) => {
                        let index_num =
                            self.interpret(*index).to_number_obj().get_number_value() as usize;

                        if let Some(var_obj) = self.interpret_mut(*var.clone()) {
                            var_obj.set_list_index(index_num, value_obj.clone());
                        }
                    }
                    _ => {}
                }

                Object::Null
            }

            Tree::Fn { name, args, body } => {
                let args_names: Vec<(String, Object)> = args
                    .iter()
                    .filter_map(|arg| match arg {
                        Tree::Ident(var) => Some((var.clone(), Object::Null)),
                        Tree::Assign(var, expr) => {
                            if let Tree::Ident(name) = *var.clone() {
                                Some((name, self.interpret(*expr.clone())))
                            } else {
                                None
                            }
                        }
                        _ => None,
                    })
                    .collect();

                // Return `Object::Invalid` if the argument extraction fails
                if args_names.len() != args.len() {
                    return Object::Invalid;
                }

                // Create and set the function object in the environment
                let function = Object::Fn {
                    name: name.clone(),
                    args: args_names,
                    body,
                };
                self.set_var(name.clone(), function.clone());

                function
            }

            Tree::FnCall {
                name,
                args: call_args,
            } => {
                // Attempt to retrieve the function object
                let obj = self.interpret(Tree::Ident(name));
                if let Object::Fn { args, body, .. } = obj {
                    // Enter a new scope for the function call
                    self.enter_scope();

                    // Bind default arguments and interpret call arguments
                    for (i, (arg_name, default_value)) in args.iter().enumerate() {
                        let value = if i < call_args.len() {
                            self.interpret(call_args[i].clone())
                        } else {
                            default_value.clone()
                        };
                        self.set_var(arg_name.clone(), value);
                    }

                    // Execute the function body
                    let result = self.evaluate_block_with_scope(&body);

                    // Return result or Object::Null
                    return match result {
                        Object::Ret(expr) => *expr,
                        _ => Object::Null,
                    };
                }

                Object::Invalid
            }

            Tree::If {
                expr,
                body,
                els,
                els_ifs,
            } => {
                // Evaluate the main `if` condition
                self.enter_scope();
                if self.interpret(*expr).to_bool_obj().get_bool_value() {
                    return self.body_block(&body);
                }

                // Evaluate any `else if` conditions
                for elsif in els_ifs {
                    if let Tree::ElsIf { expr, body } = elsif {
                        if self.interpret(*expr).to_bool_obj().get_bool_value() {
                            return self.body_block(&body);
                        }
                    }
                }

                // If no conditions are true, evaluate the `else` block if it exists
                if !els.is_empty() {
                    return self.body_block(&els);
                }
                self.exit_scope();
                Object::Null
            }

            Tree::While { expr, body } => {
                self.enter_scope();

                while self.interpret(*expr.clone()).to_bool_obj().get_bool_value() {
                    if let Some(ret) = self.execute_body(&body) {
                        self.exit_scope();
                        return ret;
                    }
                }

                self.exit_scope();
                Object::Null
            }

            Tree::For {
                ref var,
                expr,
                ref body,
            } => {
                let obj = self.interpret(*expr);
                let iter: Box<dyn Iterator<Item = Object>> = match obj {
                    Object::Range(start, end) => Box::new(
                        ((start as i32)..(end as i32)).map(|n: i32| Object::Number(n as f64)),
                    ),
                    Object::String(ref string) => {
                        Box::new(string.chars().map(|c| Object::String(c.to_string())))
                    }
                    Object::List(list) => Box::new(list.into_iter()),
                    _ => return Object::Null,
                };

                self.enter_scope();
                for item in iter {
                    self.set_var(var.to_string(), item);
                    if let Some(ret) = self.execute_body(body) {
                        self.exit_scope();
                        return ret;
                    }
                }
                self.exit_scope();
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
            Tree::Ident(name) => self.get_var(name), // Return a mutable reference to the variable
            Tree::ListCall(list, index) => {
                let index_num = self.interpret(*index).to_number_obj().get_number_value() as usize;
                if let Some(list_obj) = self.interpret_mut(*list) {
                    // Get a mutable reference to the object at the specified index in the list
                    list_obj.get_list_index_mut(index_num)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn execute_body(&mut self, body: &Vec<Tree>) -> Option<Object> {
        for stmt in body.clone() {
            if let Object::Ret(expr) = self.interpret(stmt) {
                return Some(Object::Ret(expr));
            }
        }
        None
    }

    #[inline(always)]
    fn evaluate_block_with_scope(&mut self, body: &Vec<Tree>) -> Object {
        self.enter_scope();
        let result = self.body_block(body);
        self.exit_scope();
        result
    }

    fn body_block(&mut self, body: &Vec<Tree>) -> Object {
        let mut result = Object::Null;

        for stmt in body.clone() {
            result = self.interpret(stmt);

            if let Object::Ret(expr) = result {
                self.exit_scope();
                return Object::Ret(expr);
            }
        }

        self.exit_scope();
        result
    }
}
