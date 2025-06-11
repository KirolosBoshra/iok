use crate::std_native;
use crate::{lexer::Lexer, lexer::TokenType, object::Object, parser::Parser, parser::Tree};
use core::iter::Iterator;
use rustc_hash::FxHashMap;
use std::{env, fs::File, io::Read, path::Path};

// default dir name for std libs
const STD_DIR: &str = "std";

#[derive(Debug)]
pub struct Interpreter {
    scopes: Vec<FxHashMap<String, Object>>,
    current_path: String,
    std_path: String,
}

impl Interpreter {
    pub fn new(current_path: String, std: Option<String>) -> Self {
        let std_path = if let Some(path) = std {
            path
        } else {
            env::current_exe()
                .expect("Can't get exe path")
                .parent()
                .expect("No parent directory")
                .join(STD_DIR)
                .to_str()
                .unwrap()
                .to_string()
        };

        let mut base_scope = FxHashMap::default();
        base_scope.insert(
            "write".to_string(),
            Object::NativeFn {
                name: "write".to_string(),
                function: std_native::native_write,
            },
        );
        base_scope.insert(
            "exit".to_string(),
            Object::NativeFn {
                name: "exit".to_string(),
                function: std_native::native_exit,
            },
        );

        base_scope.insert(
            "__get_var_from_str".to_string(),
            Object::NativeFn {
                name: "__get_var_from_str".to_string(),
                function: std_native::get_var_from_str,
            },
        );

        Self {
            scopes: vec![base_scope],
            current_path,
            std_path,
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

    pub fn get_var(&mut self, name: &str) -> Option<&mut Object> {
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
                                *existing_value = value_obj.clone();
                            }
                        }
                    }
                    Tree::ListCall(var, index) => {
                        let index_num =
                            self.interpret(&index).to_number_obj().get_number_value() as usize;

                        if let Some(var_obj) = self.interpret_mut(&var) {
                            var_obj.set_list_index(index_num, value_obj.clone());
                        }
                    }
                    Tree::MemberAccess { .. } => {
                        let field = self.interpret_mut(var).unwrap();
                        *field = value_obj.clone();
                    }

                    _ => {}
                }

                value_obj
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
                let var = self.get_var(name);
                if var.is_some() {
                    let obj = var.unwrap().clone();
                    self.call_function(&obj, call_args, None)
                } else {
                    println!("{name} is not a function");
                    Object::Null
                }
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
                match &**member {
                    Tree::Ident(name) => {
                        return target_object
                            .get_field(name)
                            .unwrap_or(&Object::Null)
                            .clone();
                    }

                    Tree::FnCall { name, args } => {
                        let method = match target_object {
                            Object::String(_) | Object::List(_) => {
                                // this BS but who cares
                                match &**name {
                                    "len" => return Object::Number(target_object.get_len() as f64),
                                    "push" => {
                                        let value = self.interpret(&args[0]);
                                        let target_mut = self.interpret_mut(target).unwrap();
                                        if args.len() == 1 {
                                            target_mut.push(value)
                                        } else {
                                            println!("Expected 1 arg found {}", args.len());
                                            return Object::Null;
                                        }
                                    }
                                    "pop" => {
                                        let target_mut = self.interpret_mut(target).unwrap();
                                        return target_mut.pop();
                                    }
                                    _ => {}
                                }
                                Object::Null
                            }
                            Object::Instance { ref struct_def, .. } => {
                                if let Object::StructDef { methods, .. } = &**struct_def {
                                    return self.call_function(
                                        methods.get(name).unwrap(),
                                        args,
                                        Some(&target_object),
                                    );
                                } else {
                                    Object::Null
                                }
                            }
                            Object::StructDef { ref methods, .. } => {
                                return self.call_function(
                                    methods.get(name).unwrap(),
                                    args,
                                    Some(&target_object),
                                );
                            }
                            Object::NameSpace {
                                ref namespace,
                                name: ref namespace_name,
                            } => {
                                return self.call_function(
                                    namespace.get(name).expect(
                                        format!(
                                            "function {name} doesn't exist in {namespace_name}",
                                        )
                                        .as_str(),
                                    ),
                                    args,
                                    Some(&target_object),
                                );
                            }
                            _ => Object::Null,
                        };
                        return method;
                    }

                    _ => Object::Null,
                };

                Object::Null
            }

            Tree::Import { path, alias } => {
                if let Tree::MemberAccess { .. } = &**path {
                    let flat_path = self.flatten_path(path);
                    let root_path = format!("{}/{}.iok", self.std_path, flat_path[0]);
                    let root_namespace = self.import_file_to_namespace(&root_path);

                    let mut scope = root_namespace;

                    let mut current_obj = Object::Null;
                    for (i, seg) in flat_path.iter().enumerate().skip(1) {
                        let val = scope
                            .get(seg)
                            .unwrap_or_else(|| {
                                panic!("`{}` not found in `{}`", seg, flat_path[..i].join("::"))
                            })
                            .clone();
                        if i < flat_path.len() - 1 {
                            match val {
                                Object::NameSpace { namespace, .. } => {
                                    scope = *namespace; // enter that namespace
                                }
                                _ => panic!("`{}` is not a namespace", seg),
                            }
                        } else {
                            current_obj = val;
                        }
                    }
                    let bind_name = alias.as_deref().unwrap_or(&flat_path.last().unwrap());

                    self.set_var(&bind_name, current_obj);
                    return Object::Null;
                }

                let file_path = self.resolve_import_path(&**path);
                let namespace = self.import_file_to_namespace(&file_path);

                if let Some(name) = alias {
                    let bind_name = name.as_str();
                    let obj = Object::NameSpace {
                        name: bind_name.to_string(),
                        namespace: Box::new(namespace),
                    };
                    self.set_var(bind_name, obj);
                } else {
                    self.import_namespace_into_scope(namespace);
                }
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
                if let Object::NameSpace { namespace, .. } = obj {
                    for (name, value) in namespace.iter() {
                        self.set_var(name, value.clone());
                    }
                } else {
                    self.set_var("self", obj.clone());
                }
            }

            // Execute the function body
            let result = self.eval_block(&body);
            self.exit_scope();
            // Return result or Object::Null
            return match result {
                Object::Ret(expr) => *expr,
                _ => Object::Null,
            };
        } else if let Object::NativeFn { function, .. } = function {
            let mut args_objects = vec![];
            call_args.iter().for_each(|arg| {
                args_objects.push(self.interpret(arg));
            });
            return function(args_objects, self);
        }
        Object::Null
    }

    fn resolve_import_path(&self, path: &Tree) -> String {
        let mut path_str = match path {
            Tree::String(p) => self.current_path.to_string() + "\\" + &**p,
            Tree::Ident(lib) => self.std_path.to_string() + "/" + lib + ".iok",
            _ => panic!("Expected Path or Lib name"),
        };
        if cfg!(windows) {
            path_str = path_str.replace("/", "\\");
        }

        path_str
    }

    fn flatten_path(&self, path: &Tree) -> Vec<String> {
        match path {
            Tree::Ident(name) => vec![name.clone()],
            Tree::MemberAccess { target, member } => {
                let mut parts = self.flatten_path(target);
                if let Tree::Ident(m) = &**member {
                    parts.push(m.clone());
                    parts
                } else {
                    panic!("Import path member must be identifier");
                }
            }
            _ => panic!("Invalid import path: {:?}", path),
        }
    }
    fn import_file_to_namespace(&self, file_path: &String) -> FxHashMap<String, Object> {
        let parsed_trees = self.generate_ast(file_path);
        let parent_path = Path::new(file_path)
            .canonicalize()
            .expect("Can't get path")
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        self.eval_namespace(parent_path, &parsed_trees)
    }

    fn import_namespace_into_scope(&mut self, namespace: FxHashMap<String, Object>) {
        for (name, value) in namespace {
            self.set_var(&name, value);
        }
    }
    fn generate_ast(&self, file_path: &String) -> Vec<Tree> {
        let mut input = String::new();

        let mut file = File::open(&file_path).expect("Can't locate lib");
        file.read_to_string(&mut input).expect("can't read file");
        input = input.trim_end().to_string();

        let mut lexer = Lexer::new(&input);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);

        let parsed_tree = parser.parse_tokens();
        parsed_tree
    }
    fn eval_namespace(&self, path: String, parsed_trees: &Vec<Tree>) -> FxHashMap<String, Object> {
        let mut namespace = FxHashMap::default();
        let mut mod_interpreter = Interpreter::new(path, Option::Some(self.std_path.clone()));
        parsed_trees.iter().for_each(|ast| {
            mod_interpreter.interpret(ast);
        });

        if let Some(scope) = mod_interpreter.scopes.first() {
            for (n, value) in scope {
                namespace.insert(n.clone(), value.clone());
            }
        }
        namespace
    }
}
