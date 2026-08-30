use crate::ffi;
use crate::interner::{intern, resolve};
use crate::std_native;
use crate::{
    lexer::Lexer, lexer::Loc, lexer::TokenType, logger::ErrorType, logger::Logger, object::Object,
    parser::Node, parser::Parser, parser::Tree,
};
use core::iter::Iterator;
use lazy_static::lazy_static;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::{env, fs::File, io::Read, path::Path, rc::Rc};

// default dir name for std libs
const STD_DIR: &str = "std";

// canonicalize() returns \\?\ verbatim paths which don't resolve "..";
// strip the prefix so the OS normalizes ParentDir normally
fn normalize_import_path(path: &str) -> String {
    path.trim_start_matches(r"\\?\").to_string()
}

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
    vars: Vec<Vec<Object>>,
    trail: Vec<ScopeFrame>,
    module_ctx: Vec<Option<Rc<FxHashMap<u32, Object>>>>,
    current_path: String,
    std_path: String,
    pub current_loc: Option<Loc>,
    pub script_args: Vec<String>,
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
    pub fn new(current_path: String, std: Option<String>, script_args: Vec<String>) -> Self {
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
        register_native!(base_scope, "eval", std_native::native_eval);
        register_native!(base_scope, "readline", std_native::native_readline);
        register_native!(
            base_scope,
            "__get_var_from_str",
            std_native::get_var_from_str
        );
        register_native!(base_scope, "__open_file", std_native::open_file);
        register_native!(base_scope, "__read_file", std_native::read_file);
        register_native!(base_scope, "__read_file_range", std_native::read_range);
        register_native!(base_scope, "__write_file", std_native::write_file);
        register_native!(
            base_scope,
            "__write_file_range",
            std_native::write_file_range
        );
        register_native!(base_scope, "__create_file", std_native::create_file);
        register_native!(base_scope, "__list_dir", std_native::list_dir);
        register_native!(base_scope, "__exists", std_native::exists);
        register_native!(base_scope, "__delete", std_native::delete_file);
        register_native!(base_scope, "__append", std_native::append_file);
        register_native!(base_scope, "__socket_connect", std_native::socket_connect);
        register_native!(base_scope, "__socket_bind", std_native::socket_bind);
        register_native!(base_scope, "__socket_accept", std_native::socket_accept);
        register_native!(base_scope, "__socket_read", std_native::socket_read);
        register_native!(
            base_scope,
            "__socket_read_bytes",
            std_native::socket_read_bytes
        );
        register_native!(base_scope, "__socket_write", std_native::socket_write);
        register_native!(base_scope, "__socket_close", std_native::socket_close);
        register_native!(
            base_scope,
            "__socket_is_connected",
            std_native::socket_is_connected
        );
        register_native!(
            base_scope,
            "__socket_local_addr",
            std_native::socket_local_addr
        );
        register_native!(
            base_scope,
            "__socket_peer_addr",
            std_native::socket_peer_addr
        );
        register_native!(base_scope, "__socket_read_all", std_native::socket_read_all);
        register_native!(base_scope, "__dlopen", std_native::dlopen);
        register_native!(base_scope, "__dlsym", std_native::dlsym);
        register_native!(base_scope, "__def_struct", std_native::def_struct);
        register_native!(base_scope, "__struct_val", std_native::struct_val);
        register_native!(base_scope, "__get_field", std_native::get_field);
        register_native!(base_scope, "__set_field", std_native::set_field);
        register_native!(base_scope, "__byref", std_native::byref);
        register_native!(base_scope, "__nullptr", std_native::null_ptr);
        // ── math
        register_native!(base_scope, "__math_sin", std_native::math_sin);
        register_native!(base_scope, "__math_cos", std_native::math_cos);
        register_native!(base_scope, "__math_tan", std_native::math_tan);
        register_native!(base_scope, "__math_asin", std_native::math_asin);
        register_native!(base_scope, "__math_acos", std_native::math_acos);
        register_native!(base_scope, "__math_atan", std_native::math_atan);
        register_native!(base_scope, "__math_atan2", std_native::math_atan2);
        register_native!(base_scope, "__math_sqrt", std_native::math_sqrt);
        register_native!(base_scope, "__math_pow", std_native::math_pow);
        register_native!(base_scope, "__math_exp", std_native::math_exp);
        register_native!(base_scope, "__math_ln", std_native::math_ln);
        register_native!(base_scope, "__math_log10", std_native::math_log10);
        register_native!(base_scope, "__math_abs", std_native::math_abs);
        register_native!(base_scope, "__math_floor", std_native::math_floor);
        register_native!(base_scope, "__math_ceil", std_native::math_ceil);
        register_native!(base_scope, "__math_round", std_native::math_round);
        register_native!(base_scope, "__math_trunc", std_native::math_trunc);
        register_native!(base_scope, "__math_sinh", std_native::math_sinh);
        register_native!(base_scope, "__math_cosh", std_native::math_cosh);
        register_native!(base_scope, "__math_tanh", std_native::math_tanh);
        register_native!(base_scope, "__math_hypot", std_native::math_hypot);
        register_native!(base_scope, "__math_min", std_native::math_min);
        register_native!(base_scope, "__math_max", std_native::math_max);
        register_native!(base_scope, "__math_clamp", std_native::math_clamp);
        register_native!(base_scope, "__math_rand", std_native::math_rand);
        register_native!(base_scope, "__math_rand_range", std_native::math_rand_range);
        // ── time
        register_native!(base_scope, "__time_now", std_native::time_now);
        register_native!(base_scope, "__time_millis", std_native::time_millis);
        register_native!(base_scope, "__time_sleep", std_native::time_sleep);
        // ── env
        register_native!(base_scope, "__env_args", std_native::env_args);
        register_native!(base_scope, "__env_raw_args", std_native::env_raw_args);
        register_native!(base_scope, "__env_var", std_native::env_var);
        register_native!(base_scope, "__env_cwd", std_native::env_cwd);
        register_native!(base_scope, "__env_set_var", std_native::env_set_var);

        let max_id = base_scope.keys().max().copied().unwrap_or(0);
        let mut vars = vec![Vec::new(); max_id as usize + 1];
        for (name, value) in base_scope {
            vars[name as usize].push(value);
        }

        Self {
            vars,
            trail: vec![ScopeFrame {
                names: vec![],
                is_fn: false,
            }],
            module_ctx: vec![None],
            current_path,
            std_path,
            current_loc: None,
            script_args,
        }
    }

    #[inline]
    fn enter_scope(&mut self) {
        self.trail.push(ScopeFrame {
            names: vec![],
            is_fn: false,
        });
    }

    #[inline]
    fn enter_fn_scope(&mut self) {
        self.trail.push(ScopeFrame {
            names: vec![],
            is_fn: true,
        });
    }

    #[inline]
    fn exit_scope(&mut self) {
        if let Some(bound) = self.trail.pop() {
            for name in bound.names {
                if let Some(stack) = self.vars.get_mut(name as usize) {
                    stack.pop();
                }
            }
        }
    }

    #[inline]
    fn set_var(&mut self, name: u32, value: Object) {
        if let Some(trail) = self.trail.last_mut() {
            if name as usize >= self.vars.len() {
                self.vars.resize(name as usize + 1, Vec::new());
            }
            self.vars[name as usize].push(value);
            trail.names.push(name);
        }
    }

    #[inline]
    pub fn get_var(&mut self, name: &u32) -> Option<&mut Object> {
        self.vars
            .get_mut(*name as usize)
            .and_then(|stack| stack.last_mut())
    }

    fn lookup_module(&self, name: &u32) -> Option<Object> {
        for ctx in self.module_ctx.iter().rev().flatten() {
            if let Some(v) = ctx.get(name) {
                return Some(v.clone());
            }
        }
        // linear scan of alias namespaces
        for stack in &self.vars {
            for obj in stack.iter().rev() {
                if let Object::NameSpace { namespace, .. } = obj {
                    if let Some(v) = namespace.get(name) {
                        return Some(v.clone());
                    }
                }
            }
        }
        None
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
            .get(*name as usize)
            .and_then(|stack| stack.get(total.checked_sub(1)?))
            .cloned()
    }

    #[inline]
    fn bin_op(&self, left: Object, op: &TokenType, right: Object) -> Object {
        use Object::{Invalid, List, Null, Number, String};
        use TokenType::*;

        match op {
            Plus => {
                if matches!(&left, Object::Dict(_)) || matches!(&right, Object::Dict(_)) {
                    return String(Rc::new(format!("{}{}", left, right)));
                }
                match (left, right) {
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
                }
            }
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
            Percent => match (left, right) {
                (Number(l), Number(r)) if r != 0.0 => Number(l % r),
                _ => Invalid,
            },
            DMultiply => match (left, right) {
                (Number(l), Number(r)) => Number(l.powf(r)),
                _ => Invalid,
            },
            BitAnd => left & right,
            BitOR => left | right,
            BitXor => left ^ right,
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
            TokenType::BitNot => {
                return match left {
                    Object::Number(n) => Object::Number(!(n as i64) as f64),
                    _ => Object::Invalid,
                };
            }
            TokenType::And => Bool(left.to_bool() && right.to_bool()),
            TokenType::Or => Bool(left.to_bool() || right.to_bool()),

            // Default case if none of the above match
            _ => Bool(false),
        }
    }

    pub fn interpret(&mut self, node: &Node) -> Object {
        match &node.tree {
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
            Tree::Dict(pairs) => {
                let map = Rc::new(RefCell::new(FxHashMap::default()));
                for (k_node, v_node) in pairs {
                    let k = self.interpret(k_node);
                    let v = self.interpret(v_node);
                    if !k.is_hashable() {
                        Logger::error(
                            "Dict key must be null/bool/number/string",
                            Some(node.loc),
                            ErrorType::RunTime,
                        );
                        continue;
                    }
                    map.borrow_mut().insert(k, v);
                }
                Object::Dict(map)
            }
            Tree::Ident(var) => self
                .get_var(var)
                .cloned()
                .or_else(|| self.lookup_module(var))
                .unwrap_or(Object::Null),
            Tree::Range(start, end) => {
                let start_obj = self.interpret(start);
                let end_obj = self.interpret(end);
                if let (Object::Number(s), Object::Number(e)) = (start_obj, end_obj) {
                    return Object::Range(s, e);
                }
                Object::Invalid
            }
            Tree::ListCall(var, index) => {
                let key = self.interpret(index);
                // Dict get: d["a"] / d[1] where key is any hashable object
                if let Some(var_obj) = self.interpret_mut(var) {
                    if let Object::Dict(d) = var_obj {
                        if !key.is_hashable() {
                            return Object::Null;
                        }
                        return d.borrow().get(&key).cloned().unwrap_or(Object::Null);
                    }
                    let index_num = key.to_f64() as usize;
                    return var_obj.get_list_index(index_num);
                } else {
                    let obj = self.interpret(var);
                    if let Object::Dict(d) = &obj {
                        if !key.is_hashable() {
                            return Object::Null;
                        }
                        return d.borrow().get(&key).cloned().unwrap_or(Object::Null);
                    }
                    let index_num = key.to_f64() as usize;
                    return obj.get_list_index(index_num);
                }
            }
            Tree::ImmCall { callee, args } => {
                let obj = self.interpret(callee);
                self.call_function(&obj, args, None, Some(node.loc))
            }
            Tree::Ret(expr) => Object::Ret(Box::new(self.interpret(expr))),
            Tree::Break => Object::Break,
            Tree::Continue => Object::Continue,
            Tree::BinOp(left, op, right) => {
                let left_obj = self.interpret(left);
                let right_obj = self.interpret(right);
                self.bin_op(left_obj, op, right_obj)
            }
            Tree::Ternary {
                cond,
                then_branch,
                else_branch,
            } => {
                if self.interpret(cond).to_bool() {
                    self.interpret(then_branch)
                } else {
                    self.interpret(else_branch)
                }
            }
            Tree::CmpOp(left, op, right) => {
                // && / || short-circuit: skip right side when left decides
                match op {
                    TokenType::And => {
                        if !self.interpret(left).to_bool() {
                            return Object::Bool(false);
                        }
                        return Object::Bool(self.interpret(right).to_bool());
                    }
                    TokenType::Or => {
                        if self.interpret(left).to_bool() {
                            return Object::Bool(true);
                        }
                        return Object::Bool(self.interpret(right).to_bool());
                    }
                    _ => {}
                }
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

                match &var.tree {
                    Tree::Ident(ref name) => {
                        if let Some(existing_value) = self.get_var(name) {
                            if matches!(existing_value, Object::Fn { .. }) {
                                return Object::Invalid;
                            }
                            *existing_value = value_obj.clone();
                        }
                    }
                    Tree::ListCall(var, index) => {
                        let key = self.interpret(&index);
                        if let Some(var_obj) = self.interpret_mut(&var) {
                            if let Object::Dict(d) = var_obj {
                                if !key.is_hashable() {
                                    Logger::error(
                                        "Dict key must be null/bool/number/string",
                                        Some(node.loc),
                                        ErrorType::RunTime,
                                    );
                                } else {
                                    d.borrow_mut().insert(key, value_obj.clone());
                                }
                            } else {
                                let index_num = key.to_f64() as usize;
                                var_obj.set_list_index(index_num, value_obj.clone());
                            }
                        }
                    }
                    Tree::MemberAccess { target, member } => {
                        if let Some(slot) = self.interpret_mut(target) {
                            if matches!(slot, Object::CStruct { .. }) {
                                if let Tree::Ident(name) = &member.tree {
                                    let updated = ffi::set_struct_field(
                                        slot,
                                        *name,
                                        &value_obj,
                                        Some(node.loc),
                                    );
                                    *slot = updated;
                                }
                            } else if let Object::Dict(d) = slot {
                                if let Tree::Ident(name) = &member.tree {
                                    let key = Object::String(Rc::new(resolve(*name)));
                                    d.borrow_mut().insert(key, value_obj.clone());
                                }
                            } else if let Tree::Ident(name) = &member.tree {
                                if let Some(field) = slot.get_field_mut(name) {
                                    *field = value_obj.clone();
                                }
                            }
                        }
                    }

                    _ => {}
                }

                value_obj
            }

            Tree::Fn { name, args, body } => {
                let args_names: Vec<(u32, Object)> = args
                    .iter()
                    .filter_map(|arg| match &arg.tree {
                        Tree::Ident(var) => Some((*var, Object::Null)),
                        Tree::Assign(var, expr) => {
                            if let Tree::Ident(name) = &var.tree {
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
                    name: name.unwrap_or_else(|| intern("")),
                    args: args_names,
                    body: Rc::new(body.to_vec()),
                };
                if let Some(n) = name {
                    self.set_var(*n, function.clone());
                }
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
                    self.call_function(&obj, call_args, None, Some(node.loc))
                } else if let Some(obj) = self.lookup_module(name) {
                    self.call_function(&obj, call_args, None, Some(node.loc))
                } else {
                    Logger::error(
                        &format!("Undefined function: {}", resolve(*name)),
                        Some(node.loc),
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
                        .find_map(|ei| match &ei.tree {
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

            Tree::Match { expr, arms, els } => {
                let value = self.interpret(expr);
                self.enter_scope();
                let result = arms
                    .iter()
                    .find_map(|(patterns, body)| {
                        patterns
                            .iter()
                            .any(|p| self.pattern_matches(p, &value))
                            .then(|| self.eval_block(body))
                    })
                    .unwrap_or_else(|| self.eval_block(els));
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

                // evaluate fields/methods in a throwaway scope: Tree::Let and
                // Tree::Fn side-effect-bind their names, which must not leak
                // into the surrounding module (a leaked `print` from any
                // struct hijacks io::println after a namespace merge)
                self.enter_scope();
                fields.iter().for_each(|field| {
                    if let Tree::Let(name, value) = &field.tree {
                        struct_fields.insert(*name, self.interpret(value));
                    }
                });

                methods.iter().for_each(|method| {
                    if let Tree::Fn {
                        name: Some(name),
                        args: _,
                        body: _,
                    } = &method.tree
                    {
                        struct_methods.insert(*name, self.interpret(method));
                    }
                });
                self.exit_scope();

                let def = Object::StructDef {
                    name: *struct_name,
                    fields: Rc::new(struct_fields),
                    methods: Rc::new(struct_methods),
                };
                self.set_var(*struct_name, def.clone());
                def
            }

            Tree::StructInit { name, fields } => {
                // fall back to the enclosing module context so struct literals
                // inside std modules (fs::File, net::Socket, ...) resolve
                let def_obj = match self.get_var(name) {
                    Some(obj) => Some(obj.clone()),
                    None => self.lookup_module(name),
                };
                let mut def = match def_obj {
                    Some(obj) => obj,
                    None => {
                        Logger::error(
                            &format!("Undefined struct: {}", resolve(*name)),
                            Some(node.loc),
                            ErrorType::RunTime,
                        );
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
                match &member.tree {
                    Tree::Ident(name) => {
                        let target_object = self.interpret(target);
                        if matches!(target_object, Object::CStruct { .. }) {
                            return ffi::struct_field(&target_object, *name);
                        }
                        if let Object::Dict(d) = &target_object {
                            // d.a sugar -> d["a"] if not a method
                            let key = Object::String(Rc::new(resolve(*name)));
                            if let Some(v) = d.borrow().get(&key) {
                                return v.clone();
                            }
                            return Object::Null;
                        }
                        if let Object::NameSpace {
                            name: mod_name,
                            ref namespace,
                        } = &target_object
                        {
                            if let Some(v) = namespace.get(name) {
                                return v.clone();
                            }
                            Logger::error(
                                &format!(
                                    "Undefined variable: {}::{}",
                                    resolve(*mod_name),
                                    resolve(*name)
                                ),
                                Some(node.loc),
                                ErrorType::RunTime,
                            );
                            return Object::Null;
                        }
                        return target_object
                            .get_field(name)
                            .unwrap_or(&Object::Null)
                            .clone();
                    }

                    Tree::FnCall { name, args } => {
                        let on_simple_var = self.interpret_mut(target).is_some_and(|t| {
                            matches!(t, Object::String(_) | Object::List(_) | Object::Dict(_))
                        });
                        if on_simple_var {
                            let arg_objs: Vec<Object> =
                                args.iter().map(|a| self.interpret(a)).collect();
                            return self
                                .interpret_mut(target)
                                .unwrap()
                                .simple_method(*name, &arg_objs, node.loc);
                        }

                        let mut target_object = self.interpret(target);
                        if let Object::String(_) | Object::List(_) | Object::Dict(_) =
                            &target_object
                        {
                            let arg_objs: Vec<Object> =
                                args.iter().map(|a| self.interpret(a)).collect();
                            return target_object.simple_method(*name, &arg_objs, node.loc);
                        }

                        if let Object::Instance { ref struct_def, .. } = &target_object {
                            if let Object::StructDef {
                                name: def_name,
                                methods,
                                ..
                            } = &**struct_def
                            {
                                let methods = methods.clone();
                                if let Some(method) = methods.get(name) {
                                    let result = self.call_function(
                                        method,
                                        args,
                                        Some(&mut target_object),
                                        Some(node.loc),
                                    );
                                    if let Some(slot) = self.interpret_mut(target) {
                                        *slot = target_object;
                                    }
                                    return result;
                                }
                                Logger::error(
                                    &format!(
                                        "Undefined method: {}::{}",
                                        resolve(*def_name),
                                        resolve(*name)
                                    ),
                                    Some(node.loc),
                                    ErrorType::RunTime,
                                );
                            }
                            return Object::Null;
                        }

                        if let Object::StructDef {
                            name: def_name,
                            ref methods,
                            ..
                        } = &target_object
                        {
                            let method = methods.get(name).cloned();
                            return match method {
                                Some(method) => self.call_function(
                                    &method,
                                    args,
                                    Some(&mut target_object),
                                    Some(node.loc),
                                ),
                                None => {
                                    Logger::error(
                                        &format!(
                                            "Undefined method: {}::{}",
                                            resolve(*def_name),
                                            resolve(*name)
                                        ),
                                        Some(node.loc),
                                        ErrorType::RunTime,
                                    );
                                    Object::Null
                                }
                            };
                        }

                        if let Object::NameSpace {
                            name: mod_name,
                            ref namespace,
                        } = &target_object
                        {
                            let method = namespace.get(name).cloned();
                            return match method {
                                Some(method) => self.call_function(
                                    &method,
                                    args,
                                    Some(&mut target_object),
                                    Some(node.loc),
                                ),
                                None => {
                                    Logger::error(
                                        &format!(
                                            "Undefined function: {}::{}",
                                            resolve(*mod_name),
                                            resolve(*name)
                                        ),
                                        Some(node.loc),
                                        ErrorType::RunTime,
                                    );
                                    Object::Null
                                }
                            };
                        }

                        Object::Null
                    }

                    _ => Object::Null,
                };

                Object::Null
            }

            Tree::Import { path, alias } => {
                if let Tree::MemberAccess { .. } = &path.tree {
                    let flat_path = self.flatten_path(path);
                    let root_path = format!(
                        "{}/{}.iok",
                        self.std_path.trim_end_matches(['/', '\\']),
                        resolve(flat_path[0])
                    );
                    let root_namespace = self.import_file_to_namespace(&root_path);

                    let mut scope = root_namespace;

                    let mut current_obj = Object::Null;
                    let mut failed = false;
                    for (i, seg) in flat_path.iter().enumerate().skip(1) {
                        let Some(val) = scope.get(seg).cloned() else {
                            let joined = flat_path[..i]
                                .iter()
                                .map(|id| resolve(*id))
                                .collect::<Vec<_>>()
                                .join("::");
                            Logger::error(
                                &format!("`{}` not found in `{}`", resolve(*seg), joined),
                                Some(node.loc),
                                ErrorType::RunTime,
                            );
                            failed = true;
                            break;
                        };
                        if i < flat_path.len() - 1 {
                            match val {
                                Object::NameSpace { namespace, .. } => {
                                    scope = namespace.as_ref().clone(); // enter that namespace
                                }
                                _ => {
                                    Logger::error(
                                        &format!("`{}` is not a namespace", resolve(*seg)),
                                        Some(node.loc),
                                        ErrorType::RunTime,
                                    );
                                    failed = true;
                                    break;
                                }
                            }
                        } else {
                            current_obj = val;
                        }
                    }
                    if failed {
                        return Object::Null;
                    }
                    let leaf_id = *flat_path.last().unwrap();
                    let bind_name = alias.unwrap_or(leaf_id);

                    // member import of a function also injects its sibling bindings
                    // so `import std::io::println` pulls `print`/`format` etc. without
                    // polluting the whole `std` namespace.
                    if matches!(current_obj, Object::Fn { .. }) {
                        let scope_clone = scope.clone();
                        for (k, v) in scope_clone.iter() {
                            if *k == leaf_id {
                                continue;
                            }
                            if self.get_var(k).is_none() && self.lookup_module(k).is_none() {
                                self.set_var(*k, v.clone());
                            }
                        }
                    }

                    self.set_var(bind_name, current_obj);
                    return Object::Null;
                }

                let file_path = self.resolve_import_path(&**path);
                let namespace = self.import_file_to_namespace(&file_path);

                if let Some(name) = *alias {
                    let obj = Object::NameSpace {
                        name,
                        namespace: Rc::new(namespace),
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

    fn pattern_matches(&mut self, pattern: &Node, value: &Object) -> bool {
        match &pattern.tree {
            Tree::Number(n) => value == &Object::Number(*n),
            Tree::String(s) => value == &Object::String(Rc::clone(s)),
            Tree::Bool(b) => value == &Object::Bool(*b),
            Tree::Empty() => value == &Object::Null,
            Tree::Range(s, e) => {
                let (Object::Number(start), Object::Number(end)) =
                    (self.interpret(s), self.interpret(e))
                else {
                    return false;
                };
                matches!(value, Object::Number(n) if *n >= start && *n < end)
            }
            Tree::Ident(id) => self
                .get_var(id)
                .cloned()
                .or_else(|| self.lookup_module(id))
                .is_some_and(|v| value == &v),
            _ => false,
        }
    }

    // A Helper Method to mut Objects
    #[inline]
    fn interpret_mut(&mut self, node: &Node) -> Option<&mut Object> {
        match &node.tree {
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
                    if let Tree::Ident(field_name) = &member.tree {
                        return target_obj.get_field_mut(field_name);
                    }
                }
                None
            }

            _ => None,
        }
    }

    fn eval_block(&mut self, body: &[Node]) -> Object {
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
        call_args: &Vec<Node>,
        mut slf: Option<&mut Object>,
        loc: Option<Loc>,
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
            if let Some(obj) = slf.as_deref_mut() {
                if let Object::NameSpace { namespace, .. } = obj {
                    self.module_ctx.push(Some(Rc::clone(namespace)));
                } else {
                    self.module_ctx.push(None);
                    self.set_var(*M_SELF, obj.clone());
                }
            } else {
                self.module_ctx.push(None);
            }

            // Execute the function body
            let result = self.eval_block(&body);
            self.module_ctx.pop();
            // Write mutated `self` back to the caller's instance.
            if let Some(obj) = slf {
                if !matches!(obj, Object::NameSpace { .. }) {
                    if let Some(slf_slot) = self.get_var(&*M_SELF) {
                        *obj = std::mem::take(slf_slot);
                    }
                }
            }
            self.exit_scope();
            // Return result or Object::Null
            return match result {
                Object::Ret(expr) => *expr,
                _ => Object::Null,
            };
        } else if let Object::ForeignFn {
            symbol, sig, cif, ..
        } = function
        {
            let mut args_objects = vec![];
            call_args.iter().for_each(|arg| {
                args_objects.push(self.interpret(arg));
            });
            return ffi::call_foreign(sig, cif, *symbol, &args_objects, loc);
        } else if let Object::NativeFn { function, .. } = function {
            self.current_loc = loc;
            let mut args_objects = vec![];
            call_args.iter().for_each(|arg| {
                args_objects.push(self.interpret(arg));
            });
            return function(args_objects, self);
        }
        Object::Null
    }

    fn resolve_import_path(&self, path: &Node) -> String {
        let mut path_str = match &path.tree {
            Tree::String(p) => self.current_path.to_string() + "\\" + &**p,
            Tree::Ident(lib) => self.std_path.to_string() + "/" + &resolve(*lib) + ".iok",
            _ => panic!("Expected Path or Lib name"),
        };
        if cfg!(windows) {
            path_str = path_str.replace("/", "\\");
        }
        normalize_import_path(&path_str)
    }

    fn flatten_path(&self, path: &Node) -> Vec<u32> {
        match &path.tree {
            Tree::Ident(name) => vec![*name],
            Tree::MemberAccess { target, member } => {
                let mut parts = self.flatten_path(target);
                if let Tree::Ident(m) = &member.tree {
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
        let mut input = String::new();
        let mut file = File::open(file_path).expect("Can't locate lib");
        file.read_to_string(&mut input).expect("can't read file");
        input = input.trim_end().to_string();

        // push BEFORE lexing so Loc gets this file's source id
        Logger::push_source(file_path, &input);
        let parsed_trees = self.parse_source(&input);
        let parent_path = Path::new(file_path)
            .canonicalize()
            .expect("Can't get path")
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let namespace = self.eval_namespace(parent_path, &parsed_trees);
        Logger::pop_source();
        namespace
    }

    fn import_namespace_into_scope(&mut self, namespace: FxHashMap<u32, Object>) {
        for (name, value) in namespace {
            self.set_var(name, value);
        }
    }

    fn parse_source(&self, source: &str) -> Vec<Node> {
        let input = source.to_string();
        let mut lexer = Lexer::new(&input);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse_tokens()
    }

    pub fn eval(&mut self, source: &str) -> Object {
        let mut result = Object::Null;
        Logger::push_source("eval", source);
        for tree in self.parse_source(source) {
            result = self.interpret(&tree);
            if let Object::Ret(_) | Object::Break | Object::Continue = result {
                break;
            }
        }
        Logger::pop_source();
        result
    }
    fn eval_namespace(&self, path: String, parsed_trees: &Vec<Node>) -> FxHashMap<u32, Object> {
        let mut namespace = FxHashMap::default();
        let mut mod_interpreter = Interpreter::new(
            path,
            Option::Some(self.std_path.clone()),
            self.script_args.clone(),
        );
        parsed_trees.iter().for_each(|ast| {
            mod_interpreter.interpret(ast);
        });

        for (id, stack) in mod_interpreter.vars.iter().enumerate() {
            if let Some(value) = stack.first() {
                namespace.insert(id as u32, value.clone());
            }
        }
        namespace
    }
}
