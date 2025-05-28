use crate::{lexer::TokenType, object::Object, parser::Tree};
use core::iter::Iterator;
use rustc_hash::FxHashMap;

#[derive(Debug)]
pub struct Interpreter {
    scopes: Vec<FxHashMap<String, Object>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            scopes: vec![FxHashMap::default()],
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(FxHashMap::default());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn set_var(&mut self, name: &str, value: Object) -> &mut Object {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), value);
        }
        self.get_var(name).unwrap()
    }

    fn get_var(&mut self, name: &str) -> Option<&mut Object> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(value) = scope.get_mut(name) {
                return Some(value);
            }
        }
        None
    }

    fn bin_op(&self, left: Object, op: &TokenType, right: Object) -> Object {
        use Object::{Invalid, List, Null, Number, String};
        use TokenType::*;

        match op {
            Plus => match (left, right) {
                (Number(l), Number(r)) => Number(l + r),
                (Number(l), String(r)) => String(Box::new(format!("{l}{r}"))),

                (String(mut l), String(r)) => {
                    l.push_str(&r);
                    Object::String(l) // l is already String, no new allocation
                }
                (String(l), r) => String(Box::new(format!("{l}{r}"))),

                (List(mut l), List(ref mut r)) => {
                    l.append(r);
                    List(l)
                }

                (List(mut l), r) => {
                    l.push(r);
                    List(l)
                }
                _ => Null,
            },
            Minus => match (left, right) {
                (Number(l), Number(r)) => Number(l - r),

                (Object::String(mut s), Object::Number(n)) => {
                    let n = n as usize;
                    // count total chars
                    let total = s.chars().count();
                    if n >= total {
                        s.clear();
                    } else {
                        // find byte index of the cut point
                        if let Some((byte_idx, _)) = s.char_indices().nth(total - n) {
                            s.truncate(byte_idx);
                        }
                    }
                    Object::String(s)
                }
                // String - String: remove all occurrences of the second string
                (Object::String(s), Object::String(r)) => {
                    // perform a global replace
                    let result = s.replace(&*r, "");
                    Object::String(Box::new(result))
                }
                _ => Null,
            },
            Multiply => match (left, right) {
                (Number(l), Number(r)) => Number(l * r),
                (Number(l), String(r)) | (String(r), Number(l)) => {
                    String(Box::new(r.repeat(l as usize)))
                }
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

    fn cmp_op(&self, left: Object, op: &TokenType, right: Object) -> Object {
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
                Bool(left.to_bool_obj().get_bool_value() && right.to_bool_obj().get_bool_value())
            }
            TokenType::Or => {
                Bool(left.to_bool_obj().get_bool_value() || right.to_bool_obj().get_bool_value())
            }

            // Default case if none of the above match
            _ => Bool(false),
        }
    }

    pub fn interpret(&mut self, tree: &Tree) -> Object {
        match tree {
            Tree::Empty() => Object::Null,
            Tree::Number(num) => Object::Number(*num),
            Tree::Bool(b) => Object::Bool(*b),
            Tree::String(s) => Object::String(s.clone()),
            Tree::List(list) => {
                let mut buf = vec![];
                list.iter().for_each(|item| {
                    buf.push(self.interpret(item));
                });
                Object::List(buf)
            }
            Tree::Ident(var) => self.get_var(&var).unwrap_or(&mut Object::Null).clone(),
            Tree::Range(start, end) => {
                let start_obj = self.interpret(start);
                let end_obj = self.interpret(end);
                if let (Object::Number(s), Object::Number(e)) = (start_obj, end_obj) {
                    return Object::Range(s, e);
                }
                Object::Invalid
            }
            Tree::ListCall(var, index) => self
                .interpret(var)
                .get_list_index(self.interpret(index).to_number_obj().get_number_value() as usize),
            Tree::Ret(expr) => Object::Ret(Box::new(self.interpret(expr))),
            Tree::BinOp(left, op, right) => {
                let left_obj = self.interpret(left);
                let right_obj = self.interpret(right);
                self.bin_op(left_obj, op, right_obj)
            }
            Tree::CmpOp(left, op, right) => {
                let left_obj = self.interpret(left);
                let right_obj = self.interpret(right);
                self.cmp_op(left_obj, op, right_obj)
            }

            Tree::Let(var, value) => {
                let v_obj = self.interpret(value);
                let value_obj = if let Object::Ret(expr) = v_obj {
                    *expr
                } else {
                    v_obj
                };
                self.set_var(&var, value_obj);
                Object::Null
            }

            Tree::Assign(var, value) => {
                let mut value_obj = self.interpret(value);

                if let Object::Ret(expr) = &mut value_obj {
                    value_obj = std::mem::take(expr);
                }

                match &**var {
                    Tree::Ident(ref name) => {
                        for scope in self.scopes.iter_mut().rev() {
                            if let Some(existing_value) = scope.get_mut(name) {
                                if matches!(existing_value, Object::Fn { .. }) {
                                    return Object::Invalid;
                                }
                                *existing_value = value_obj;
                                return Object::Null;
                            }
                        }
                    }
                    Tree::ListCall(var, index) => {
                        let index_num =
                            self.interpret(&index).to_number_obj().get_number_value() as usize;

                        if let Some(var_obj) = self.interpret_mut(&var) {
                            var_obj.set_list_index(index_num, value_obj);
                        }
                    }
                    Tree::MemberAccess { .. } => {
                        let field = self.interpret_mut(var).unwrap();
                        *field = value_obj;
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
                            if let Tree::Ident(name) = &**var {
                                Some((name.to_string(), self.interpret(&expr)))
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
                    name: name.to_string(),
                    args: args_names,
                    body: body.to_vec(),
                };
                self.set_var(&name, function).clone()
            }

            Tree::FnCall {
                name,
                args: call_args,
            } => {
                // Attempt to retrieve the function object
                let obj = self.get_var(name).unwrap().clone();
                self.call_function(&obj, call_args, None)
            }

            Tree::If {
                expr,
                body,
                els,
                els_ifs,
            } => {
                self.enter_scope();
                let result = if self.interpret(expr).to_bool_obj().get_bool_value() {
                    self.eval_block(body)
                } else {
                    els_ifs
                        .iter()
                        .find_map(|ei| match ei {
                            Tree::ElsIf { expr, body }
                                if self.interpret(expr).to_bool_obj().get_bool_value() =>
                            {
                                Some(self.eval_block(body))
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| self.eval_block(els))
                };
                self.exit_scope();
                result
            }

            Tree::While { expr, body } => {
                self.enter_scope();

                while self.interpret(expr).to_bool_obj().get_bool_value() {
                    if let Object::Ret(v) = self.eval_block(&body) {
                        self.exit_scope();
                        return *v;
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
                let obj = self.interpret(expr);
                let iter: Box<dyn Iterator<Item = Object>> = match obj {
                    Object::Range(start, end) => Box::new(
                        ((start as i32)..(end as i32)).map(|n: i32| Object::Number(n as f64)),
                    ),
                    Object::String(ref string) => Box::new(
                        string
                            .chars()
                            .map(|c| Object::String(Box::new(c.to_string()))),
                    ),
                    Object::List(list) => Box::new(list.into_iter()),
                    _ => return Object::Null,
                };

                self.enter_scope();
                for item in iter {
                    self.set_var(var, item);
                    if let Object::Ret(v) = self.eval_block(body) {
                        self.exit_scope();
                        return *v;
                    }
                }
                self.exit_scope();
                Object::Null
            }

            Tree::StructDef {
                name: struct_name,
                fields,
                methods,
            } => {
                let mut struct_fields = FxHashMap::default();
                let mut struct_methods = FxHashMap::default();

                fields.iter().for_each(|field| {
                    if let Tree::Let(name, value) = field {
                        struct_fields.insert(name.to_string(), self.interpret(value));
                    }
                });

                methods.iter().for_each(|method| {
                    if let Tree::Fn {
                        name,
                        args: _,
                        body: _,
                    } = method
                    {
                        struct_methods.insert(name.to_string(), self.interpret(method));
                    }
                });

                let def = Object::StructDef {
                    name: struct_name.clone(),
                    fields: Box::new(struct_fields),
                    methods: Box::new(struct_methods),
                };
                self.set_var(struct_name, def.clone());
                def
            }

            Tree::StructInit { name, fields } => {
                let mut def = self.get_var(name).unwrap().clone();
                if let Object::StructDef {
                    name: _,
                    fields: ref mut def_fields,
                    methods: _,
                } = def
                {
                    fields.iter().for_each(|(field, value)| {
                        def_fields.insert(field.to_string(), self.interpret(value));
                    });
                    let f = *def_fields.clone();
                    return Object::Instance {
                        struct_def: Box::new(def),
                        fields: f,
                    };
                } else {
                    Object::Null
                }
            }

            Tree::MemberAccess { target, member } => {
                let target_object = self.interpret(target);

                if let Object::Instance {
                    ref struct_def,
                    ref fields,
                } = target_object
                {
                    if let Object::StructDef {
                        name: _,
                        fields: _,
                        ref methods,
                    } = **struct_def
                    {
                        match &**member {
                            Tree::Ident(name) => {
                                return fields.get(name).unwrap_or(&Object::Null).clone();
                            }
                            Tree::FnCall { name, args } => {
                                return self.call_function(
                                    methods.get(name).unwrap(),
                                    args,
                                    Some(&target_object),
                                );
                            }
                            _ => {}
                        }
                    }
                }

                Object::Null
            }

            // TODO Move Static Access to separate fn
            // Tree::StaticAccess { target, member } => {}
            Tree::Exit(code) => {
                let exit_code = match self.interpret(code) {
                    Object::Number(num) => num as i32,
                    Object::Null => 0,
                    _ => -1,
                };
                std::process::exit(exit_code);
            }

            Tree::Dbg(expr) => {
                let expr_obj = self.interpret(expr);
                println!("{expr_obj}");
                Object::Null
            }

            _ => Object::Null,
        }
    }

    // A Helper Method to mut Objects
    fn interpret_mut(&mut self, tree: &Tree) -> Option<&mut Object> {
        match tree {
            Tree::Ident(name) => self.get_var(&name), // Return a mutable reference to the variable
            Tree::ListCall(list, index) => {
                let index_num = self.interpret(index).to_number_obj().get_number_value() as usize;
                if let Some(list_obj) = self.interpret_mut(list) {
                    // Get a mutable reference to the object at the specified index in the list
                    list_obj.get_list_index_mut(index_num)
                } else {
                    None
                }
            }
            Tree::MemberAccess { target, member } => {
                let target_obj = self.interpret_mut(target).unwrap();

                if let Tree::Ident(field_name) = &**member {
                    return target_obj.get_field_mut(field_name);
                }
                None
            }

            _ => None,
        }
    }

    fn eval_block(&mut self, body: &[Tree]) -> Object {
        let mut result = Object::Null;
        for stmt in body {
            result = self.interpret(stmt);
            if let Object::Ret(_) = result {
                break;
            }
        }
        result
    }
    pub fn call_function(
        &mut self,
        function: &Object,
        call_args: &Vec<Tree>,
        slf: Option<&Object>,
    ) -> Object {
        if let Object::Fn { args, body, .. } = function {
            self.enter_scope();
            // Bind default arguments and interpret call arguments
            for (i, (arg_name, default_value)) in args.iter().enumerate() {
                let value = if i < call_args.len() {
                    self.interpret(&call_args[i])
                } else {
                    default_value.clone()
                };
                self.set_var(&arg_name, value);
            }
            if let Some(obj) = slf {
                self.set_var("self", obj.clone());
            }

            // Execute the function body
            let result = self.eval_block(&body);
            self.exit_scope();
            // Return result or Object::Null
            return match result {
                Object::Ret(expr) => *expr,
                _ => Object::Null,
            };
        }
        Object::Null
    }
}
