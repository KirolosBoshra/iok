use crate::{lexer::TokenType, object::Object, parser::Tree};
use core::iter::Iterator;
use std::collections::HashMap;

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
                let v_obj = self.interpret(*value.clone());
                let value_obj = if let Object::Ret(expr) = v_obj {
                    *expr
                } else {
                    v_obj
                };
                match *var {
                    Tree::Ident(name) => {
                        for scope in self.scopes.iter_mut().rev() {
                            if scope.contains_key(&name) {
                                match scope.get(&name).unwrap() {
                                    Object::Fn {
                                        name: _,
                                        args: _,
                                        body: _,
                                    } => {
                                        return Object::Invalid;
                                    }
                                    _ => {
                                        scope.insert(name, value_obj.clone());
                                        return value_obj;
                                    }
                                }
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

            Tree::Fn { name, args, body } => {
                // Extract argument names from the `args` vector
                let args_names: Vec<(String, Object)> = args
                    .iter()
                    .filter_map(|arg| {
                        if let Tree::Ident(var) = arg {
                            Some((var.clone(), Object::Null))
                        } else if let Tree::Assign(var, expr) = arg {
                            if let Tree::Ident(name) = *var.clone() {
                                Some((name, self.interpret(*expr.clone())))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                // Return `Object::Invalid` if any argument is not an identifier
                if args_names.len() != args.len() {
                    return Object::Invalid;
                }

                // Set the function object in the environment
                self.set_var(
                    name.clone(),
                    Object::Fn {
                        name,
                        args: args_names,
                        body,
                    },
                )
                .clone()
            }

            Tree::FnCall {
                name,
                args: call_args,
            } => {
                // Interpret the function name to get the function object
                let fn_obj = self.interpret(Tree::Ident(name));

                // Ensure the object is a function
                if let Object::Fn { args, body, .. } = fn_obj {
                    // Enter a new scope for the function call
                    self.enter_scope();

                    // Bind the function arguments to their corresponding values
                    for arg in args.clone() {
                        self.set_var(arg.0, arg.1);
                    }

                    for (i, arg) in call_args.iter().enumerate() {
                        let arg_obj = self.interpret(arg.clone());
                        if i < args.len() {
                            self.set_var(args[i].clone().0, arg_obj);
                        } else {
                            return Object::Invalid;
                        }
                    }

                    // Execute the function body
                    for stmt in body {
                        let result = self.interpret(stmt);
                        if let Object::Ret(expr) = result {
                            self.exit_scope();
                            return *expr;
                        }
                    }

                    // Exit the scope after the function body is executed
                    self.exit_scope();
                    Object::Null
                } else {
                    Object::Invalid
                }
            }

            Tree::If {
                expr,
                body,
                els,
                els_ifs,
            } => {
                // Interpret the condition expression
                let expr_obj = self.interpret(*expr);

                // If the condition is true, execute the 'if' block
                if expr_obj.to_bool_obj().get_bool_value() {
                    self.enter_scope();
                    return self.body_block(&body);
                }

                // If the condition is false, check the 'else if' branches
                for elsif in els_ifs {
                    if let Tree::ElsIf {
                        expr,
                        body: elsif_body,
                    } = elsif
                    {
                        let expr_stmt_obj = self.interpret(*expr);
                        if expr_stmt_obj.to_bool_obj().get_bool_value() {
                            return self.body_block(&elsif_body);
                        }
                    }
                }

                // If no 'else if' branch was true, execute the 'else' block if it exists
                if !els.is_empty() {
                    self.enter_scope();
                    return self.body_block(&els);
                }

                Object::Null
            }

            Tree::While { expr, body } => {
                let mut expr_obj = self.interpret(*expr.clone());
                self.enter_scope();
                while expr_obj.to_bool_obj().get_bool_value() {
                    for stmt in body.clone() {
                        if let Object::Ret(expr) = self.interpret(stmt.clone()) {
                            self.exit_scope();
                            return Object::Ret(expr);
                        }
                    }
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
                            for stmt in body.clone() {
                                if let Object::Ret(expr) = self.interpret(stmt.clone()) {
                                    self.exit_scope();
                                    return Object::Ret(expr);
                                }
                            }
                        }
                        self.exit_scope();
                    }
                    Object::String(string) => {
                        self.enter_scope();
                        for c in string.chars() {
                            self.set_var(var.clone(), Object::String(c.to_string()));
                            for stmt in body.clone() {
                                if let Object::Ret(expr) = self.interpret(stmt.clone()) {
                                    self.exit_scope();
                                    return Object::Ret(expr);
                                }
                            }
                        }
                        self.exit_scope();
                    }
                    Object::List(list) => {
                        self.enter_scope();
                        for item in list {
                            self.set_var(var.clone(), item);
                            for stmt in body.clone() {
                                if let Object::Ret(expr) = self.interpret(stmt.clone()) {
                                    self.exit_scope();
                                    return Object::Ret(expr);
                                }
                            }
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
    // Change this to Option
    fn body_block(&mut self, body: &Vec<Tree>) -> Object {
        for stmt in body.clone() {
            if let Object::Ret(expr) = self.interpret(stmt.clone()) {
                self.exit_scope();
                return Object::Ret(expr);
            }
        }
        self.exit_scope();
        Object::Null
    }
}
