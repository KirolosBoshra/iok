use crate::interner::{intern, resolve};
use crate::std_native;
use crate::{
    lexer::Lexer, lexer::TokenType, logger::ErrorType, logger::Logger, object::Object,
    parser::Parser, parser::Tree,
};
use core::iter::Iterator;
use lazy_static::lazy_static;
use rustc_hash::FxHashMap;
use std::{env, fs::File, io::Read, path::Path, rc::Rc};

// default dir name for std libs
const STD_DIR: &str = "std";

lazy_static! {
    static ref M_SELF: u32 = intern("self");
}

#[derive(Debug)]
struct ScopeFrame {
    names: Vec<u32>,
    is_fn: bool,
}

#[derive(Debug)]
pub struct Interpreter {
    vars: FxHashMap<u32, Vec<Object>>,
    trail: Vec<ScopeFrame>,
    current_path: String,
    std_path: String,
}

macro_rules! register_native {
    ($scope:expr, $name:literal, $fn:path) => {
        $scope.insert(
            intern($name),
            Object::NativeFn {
                name: $name.to_string(),
                function: $fn,
            },
        );
    };
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
        register_native!(base_scope, "write", std_native::native_write);
        register_native!(base_scope, "exit", std_native::native_exit);
        register_native!(base_scope, "chr", std_native::native_chr);
        register_native!(base_scope, "__get_var_from_str", std_native::get_var_from_str);
        register_native!(base_scope, "__open_file", std_native::open_file);
        register_native!(base_scope, "__read_file", std_native::read_file);
        register_native!(base_scope, "__read_file_range", std_native::read_range);
        register_native!(base_scope, "__write_file", std_native::write_file);
        register_native!(base_scope, "__write_file_range", std_native::write_file_range);
        register_native!(base_scope, "__create_file", std_native::create_file);
        register_native!(base_scope, "__socket_connect", std_native::socket_connect);
        register_native!(base_scope, "__socket_bind", std_native::socket_bind);
        register_native!(base_scope, "__socket_accept", std_native::socket_accept);
        register_native!(base_scope, "__socket_read", std_native::socket_read);
        register_native!(base_scope, "__socket_read_bytes", std_native::socket_read_bytes);
        register_native!(base_scope, "__socket_write", std_native::socket_write);
        register_native!(base_scope, "__socket_close", std_native::socket_close);
        register_native!(
            base_scope,
            "__socket_is_connected",
            std_native::socket_is_connected
        );
        register_native!(base_scope, "__socket_local_addr", std_native::socket_local_addr);
        register_native!(base_scope, "__socket_peer_addr", std_native::socket_peer_addr);
        register_native!(base_scope, "__socket_read_all", std_native::socket_read_all);

        let vars = base_scope
            .into_iter()
            .map(|(name, value)| (name, vec![value]))
            .collect();

        Self {
            vars,
            trail: vec![ScopeFrame { names: vec![], is_fn: false }],
            current_path,
            std_path,
        }
    }

    #[inline]
    fn enter_scope(&mut self) {
        self.trail.push(ScopeFrame { names: vec![], is_fn: false });
    }

    #[inline]
    fn enter_fn_scope(&mut self) {
        self.trail.push(ScopeFrame { names: vec![], is_fn: true });
    }

    #[inline]
    fn exit_scope(&mut self) {
        if let Some(bound) = self.trail.pop() {
            for name in bound.names {
                if let Some(stack) = self.vars.get_mut(&name) {
                    stack.pop();
                    if stack.is_empty() {
                        self.vars.remove(&name);
                    }
                }
            }
        }
    }

    #[inline]
    fn set_var(&mut self, name: u32, value: Object) {
        if let Some(trail) = self.trail.last_mut() {
            self.vars.entry(name).or_default().push(value);
            trail.names.push(name);
        }
    }

    #[inline]
    pub fn get_var(&mut self, name: &u32) -> Option<&mut Object> {
        self.vars.get_mut(name).and_then(|stack| stack.last_mut())
    }

    // Resolve a variable as seen from the caller of the current function,
    // skipping every binding made inside the running function's scopes.
    pub fn get_var_from_caller_scope(&mut self, name: &u32) -> Option<Object> {
        let cutoff = self.trail.iter().rposition(|f| f.is_fn)?;
        let total: usize = self.trail[..cutoff]
            .iter()
            .map(|f| f.names.iter().filter(|n| *n == name).count())
            .sum();
        self.vars
            .get(name)
            .and_then(|stack| stack.get(total.checked_sub(1)?))
            .cloned()
    }

    #[inline]
    fn bin_op(&self, left: Object, op: &TokenType, right: Object) -> Object {
        use Object::{Invalid, List, Null, Number, String};
        use TokenType::*;

        match op {
            Plus => match (left, right) {
                (Number(l), Number(r)) => Number(l + r),
                (Number(l), String(r)) => String(Rc::new(format!("{l}{r}"))),

                (String(mut l), String(r)) => {
                    Rc::make_mut(&mut l).push_str(&r);
                    Object::String(l)
                }
                (String(l), r) => String(Rc::new(format!("{l}{r}"))),

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
                    let s_mut = Rc::make_mut(&mut s);
                    // count total chars
                    let total = s_mut.chars().count();
                    if n >= total {
                        s_mut.clear();
                    } else {
                        // find byte index of the cut point
                        if let Some((byte_idx, _)) = s_mut.char_indices().nth(total - n) {
                            s_mut.truncate(byte_idx);
                        }
                    }
                    Object::String(s)
                }
                // String - String: remove all occurrences of the second string
                (Object::String(s), Object::String(r)) => {
                    // perform a global replace
                    let result = s.replace(&*r, "");
                    Object::String(Rc::new(result))
                }
                _ => Null,
            },
            Multiply => match (left, right) {
                (Number(l), Number(r)) => Number(l * r),
                (Number(l), String(r)) | (String(r), Number(l)) => {
                    String(Rc::new(r.repeat(l as usize)))
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

    #[inline]
    fn cmp_op(&self, left: Object, op: &TokenType, right: Object) -> Object {
        use Object::{Bool, Number};

        match op {
            // Direct equality and inequality checks
            TokenType::EquEqu => return Bool(left == right),
            TokenType::NotEqu => return Bool(left != right),

            // Lazy evaluation for greater/less comparison
            TokenType::Greater | TokenType::GreatEqu | TokenType::Less | TokenType::LessEqu => {
                match (left, right) {
                    (Number(left_num), Number(right_num)) => Bool(match op {
                        TokenType::Greater => left_num > right_num,
                        TokenType::GreatEqu => left_num >= right_num,
                        TokenType::Less => left_num < right_num,
                        TokenType::LessEqu => left_num <= right_num,
                        _ => false,
                    }),
                    _ => Bool(false),
                }
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
            Tree::String(s) => Object::String(Rc::clone(s)),
            Tree::List(list) => {
                let mut buf = vec![];
                list.iter().for_each(|item| {
                    buf.push(self.interpret(item));
                });
                Object::List(buf)
            }
            Tree::Ident(var) => self.get_var(var).unwrap_or(&mut Object::Null).clone(),
            Tree::Range(start, end) => {
                let start_obj = self.interpret(start);
                let end_obj = self.interpret(end);
                if let (Object::Number(s), Object::Number(e)) = (start_obj, end_obj) {
                    return Object::Range(s, e);
                }
                Object::Invalid
            }
            Tree::ListCall(var, index) => {
                let index_num = self.interpret(index).to_f64() as usize;
                if let Some(var_obj) = self.interpret_mut(var) {
                    var_obj.get_list_index(index_num)
                } else {
                    self.interpret(var).get_list_index(index_num)
                }
            }
            Tree::Ret(expr) => Object::Ret(Box::new(self.interpret(expr))),
            Tree::Break => Object::Break,
            Tree::Continue => Object::Continue,
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
                self.set_var(*var, value_obj);
                Object::Null
            }

            Tree::Assign(var, value) => {
                let mut value_obj = self.interpret(value);

                if let Object::Ret(expr) = &mut value_obj {
                    value_obj = std::mem::take(expr);
                }

                match &**var {
                    Tree::Ident(ref name) => {
                        if let Some(existing_value) = self.get_var(name) {
                            if matches!(existing_value, Object::Fn { .. }) {
                                return Object::Invalid;
                            }
                            *existing_value = value_obj.clone();
                        }
                    }
                    Tree::ListCall(var, index) => {
                        let index_num = self.interpret(&index).to_f64() as usize;

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
                let args_names: Vec<(u32, Object)> = args
                    .iter()
                    .filter_map(|arg| match arg {
                        Tree::Ident(var) => Some((*var, Object::Null)),
                        Tree::Assign(var, expr) => {
                            if let Tree::Ident(name) = &**var {
                                Some((*name, self.interpret(&expr)))
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
                    name: *name,
                    args: args_names,
                    body: Rc::new(body.to_vec()),
                };
                self.set_var(*name, function.clone());
                function
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
                    Logger::error(
                        &format!("Undefined function: {}", resolve(*name)),
                        None,
                        ErrorType::RunTime,
                    );
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
                let result = if self.interpret(expr).to_bool() {
                    self.eval_block(body)
                } else {
                    els_ifs
                        .iter()
                        .find_map(|ei| match ei {
                            Tree::ElsIf { expr, body } if self.interpret(expr).to_bool() => {
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

                while self.interpret(expr).to_bool() {
                    match self.eval_block(&body) {
                        Object::Ret(v) => {
                            self.exit_scope();
                            return Object::Ret(v);
                        }
                        Object::Break => break,
                        _ => {}
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
                            .map(|c| Object::String(Rc::new(c.to_string()))),
                    ),
                    Object::List(list) => Box::new(list.into_iter()),
                    _ => return Object::Null,
                };

                self.enter_scope();
                self.set_var(*var, Object::Null);
                for item in iter {
                    if let Some(slot) = self.get_var(var) {
                        *slot = item;
                    }
                    match self.eval_block(body) {
                        Object::Ret(v) => {
                            self.exit_scope();
                            return Object::Ret(v);
                        }
                        Object::Break => break,
                        _ => {}
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
                        struct_fields.insert(*name, self.interpret(value));
                    }
                });

                methods.iter().for_each(|method| {
                    if let Tree::Fn {
                        name,
                        args: _,
                        body: _,
                    } = method
                    {
                        struct_methods.insert(*name, self.interpret(method));
                    }
                });

                let def = Object::StructDef {
                    name: *struct_name,
                    fields: Rc::new(struct_fields),
                    methods: Rc::new(struct_methods),
                };
                self.set_var(*struct_name, def.clone());
                def
            }

            Tree::StructInit { name, fields } => {
                let mut def = match self.get_var(name) {
                    Some(obj) => obj.clone(),
                    None => {
                        println!("Runtime Error: Undefined struct: {}", resolve(*name));
                        return Object::Null;
                    }
                };
                if let Object::StructDef {
                    name: _,
                    fields: ref mut def_fields,
                    methods: _,
                } = def
                {
                    fields.iter().for_each(|(field, value)| {
                        Rc::make_mut(def_fields).insert(*field, self.interpret(value));
                    });
                    let f = (**def_fields).clone();
                    return Object::Instance {
                        struct_def: Box::new(def),
                        fields: Rc::new(f),
                    };
                } else {
                    Object::Null
                }
            }

            Tree::MemberAccess { target, member } => {
                match &**member {
                    Tree::Ident(name) => {
                        let target_object = self.interpret(target);
                        return target_object
                            .get_field(name)
                            .unwrap_or(&Object::Null)
                            .clone();
                    }

                    Tree::FnCall { name, args } => {
                        let on_simple_var = self
                            .interpret_mut(target)
                            .is_some_and(|t| matches!(t, Object::String(_) | Object::List(_)));
                        if on_simple_var {
                            let arg_objs: Vec<Object> =
                                args.iter().map(|a| self.interpret(a)).collect();
                            return self
                                .interpret_mut(target)
                                .unwrap()
                                .simple_method(*name, &arg_objs);
                        }

                        let mut target_object = self.interpret(target);
                        if let Object::String(_) | Object::List(_) = &target_object {
                            let arg_objs: Vec<Object> =
                                args.iter().map(|a| self.interpret(a)).collect();
                            return target_object.simple_method(*name, &arg_objs);
                        }

                        if let Object::Instance { ref struct_def, .. } = &target_object {
                            if let Object::StructDef { methods, .. } = &**struct_def {
                                let methods = methods.clone();
                                if let Some(method) = methods.get(name) {
                                    let result =
                                        self.call_method(method, args, &mut target_object);
                                    if let Some(slot) = self.interpret_mut(target) {
                                        *slot = target_object;
                                    }
                                    return result;
                                }
                            }
                            return Object::Null;
                        }

                        if let Object::StructDef { ref methods, .. } = &target_object {
                            return match methods.get(name) {
                                Some(method) => {
                                    self.call_function(method, args, Some(&target_object))
                                }
                                None => Object::Null,
                            };
                        }

                        if let Object::NameSpace { ref namespace, .. } = &target_object {
                            return match namespace.get(name) {
                                Some(method) => {
                                    self.call_function(method, args, Some(&target_object))
                                }
                                None => Object::Null,
                            };
                        }

                        Object::Null
                    }

                    _ => Object::Null,
                };

                Object::Null
            }

            Tree::Import { path, alias } => {
                if let Tree::MemberAccess { .. } = &**path {
                    let flat_path = self.flatten_path(path);
                    let root_path = format!("{}/{}.iok", self.std_path, resolve(flat_path[0]));
                    let root_namespace = self.import_file_to_namespace(&root_path);

                    let mut scope = root_namespace;

                    let mut current_obj = Object::Null;
                    for (i, seg) in flat_path.iter().enumerate().skip(1) {
                        let val = scope
                            .get(seg)
                            .unwrap_or_else(|| {
                                let joined = flat_path[..i]
                                    .iter()
                                    .map(|id| resolve(*id))
                                    .collect::<Vec<_>>()
                                    .join("::");
                                panic!("`{}` not found in `{}`", resolve(*seg), joined)
                            })
                            .clone();
                        if i < flat_path.len() - 1 {
                            match val {
                                Object::NameSpace { namespace, .. } => {
                                    scope = *namespace; // enter that namespace
                                }
                                _ => panic!("`{}` is not a namespace", resolve(*seg)),
                            }
                        } else {
                            current_obj = val;
                        }
                    }
                    let bind_name = alias.map(|a| a).unwrap_or(*flat_path.last().unwrap());

                    self.set_var(bind_name, current_obj);
                    return Object::Null;
                }

                let file_path = self.resolve_import_path(&**path);
                let namespace = self.import_file_to_namespace(&file_path);

                if let Some(name) = *alias {
                    let obj = Object::NameSpace {
                        name,
                        namespace: Box::new(namespace),
                    };
                    self.set_var(name, obj);
                } else {
                    self.import_namespace_into_scope(namespace);
                }
                Object::Null
            }

            _ => Object::Null,
        }
    }

    // A Helper Method to mut Objects
    #[inline]
    fn interpret_mut(&mut self, tree: &Tree) -> Option<&mut Object> {
        match tree {
            Tree::Ident(name) => self.get_var(name), // Return a mutable reference to the variable
            Tree::ListCall(list, index) => {
                let index_num = self.interpret(index).to_f64() as usize;
                if let Some(list_obj) = self.interpret_mut(list) {
                    // Get a mutable reference to the object at the specified index in the list
                    list_obj.get_list_index_mut(index_num)
                } else {
                    None
                }
            }
            Tree::MemberAccess { target, member } => {
                if let Some(target_obj) = self.interpret_mut(target) {
                    if let Tree::Ident(field_name) = &**member {
                        return target_obj.get_field_mut(field_name);
                    }
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
            if let Object::Ret(_) | Object::Break | Object::Continue = result {
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
            self.enter_fn_scope();
            // Bind default arguments and interpret call arguments
            for (i, (arg_name, default_value)) in args.iter().enumerate() {
                let value = if i < call_args.len() {
                    self.interpret(&call_args[i])
                } else {
                    default_value.clone()
                };
                self.set_var(*arg_name, value);
            }
            if let Some(obj) = slf {
                if let Object::NameSpace { namespace, .. } = obj {
                    for (name, value) in namespace.iter() {
                        self.set_var(*name, value.clone());
                    }
                } else {
                    self.set_var(*M_SELF, obj.clone());
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

    fn call_method(&mut self, function: &Object, call_args: &Vec<Tree>, slf: &mut Object) -> Object {
        if let Object::Fn { args, body, .. } = function {
            self.enter_fn_scope();
            for (i, (arg_name, default_value)) in args.iter().enumerate() {
                let value = if i < call_args.len() {
                    self.interpret(&call_args[i])
                } else {
                    default_value.clone()
                };
                self.set_var(*arg_name, value);
            }
            self.set_var(*M_SELF, slf.clone());
            let result = self.eval_block(&body);
            // Write mutated `self` back to the caller's instance.
            if let Some(slf_slot) = self.get_var(&*M_SELF) {
                *slf = std::mem::take(slf_slot);
            }
            self.exit_scope();
            return match result {
                Object::Ret(expr) => *expr,
                _ => Object::Null,
            };
        }
        Object::Null
    }

    fn resolve_import_path(&self, path: &Tree) -> String {
        let mut path_str = match path {
            Tree::String(p) => self.current_path.to_string() + "\\" + &**p,
            Tree::Ident(lib) => self.std_path.to_string() + "/" + &resolve(*lib) + ".iok",
            _ => panic!("Expected Path or Lib name"),
        };
        if cfg!(windows) {
            path_str = path_str.replace("/", "\\");
        }

        path_str
    }

    fn flatten_path(&self, path: &Tree) -> Vec<u32> {
        match path {
            Tree::Ident(name) => vec![*name],
            Tree::MemberAccess { target, member } => {
                let mut parts = self.flatten_path(target);
                if let Tree::Ident(m) = &**member {
                    parts.push(*m);
                    parts
                } else {
                    panic!("Import path member must be identifier");
                }
            }
            _ => panic!("Invalid import path: {:?}", path),
        }
    }
    fn import_file_to_namespace(&self, file_path: &String) -> FxHashMap<u32, Object> {
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

    fn import_namespace_into_scope(&mut self, namespace: FxHashMap<u32, Object>) {
        for (name, value) in namespace {
            self.set_var(name, value);
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
    fn eval_namespace(&self, path: String, parsed_trees: &Vec<Tree>) -> FxHashMap<u32, Object> {
        let mut namespace = FxHashMap::default();
        let mut mod_interpreter = Interpreter::new(path, Option::Some(self.std_path.clone()));
        parsed_trees.iter().for_each(|ast| {
            mod_interpreter.interpret(ast);
        });

        for (n, stack) in mod_interpreter.vars.iter() {
            if let Some(value) = stack.first() {
                namespace.insert(*n, value.clone());
            }
        }
        namespace
    }
}
