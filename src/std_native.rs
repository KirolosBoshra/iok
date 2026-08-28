use crate::ffi;
use crate::file_handler::FileHandler;
use crate::interner::intern;
use crate::interpreter::Interpreter;
use crate::logger::{ErrorType, Logger};
use crate::object::Object;
use crate::socket::Socket;
use libloading::{Library, Symbol};
use std::io::Write;
use std::os::raw::c_void;
use std::rc::Rc;

pub type NativeFn = fn(Vec<Object>, &mut Interpreter) -> Object;

pub fn native_write(args: Vec<Object>, _: &mut Interpreter) -> Object {
    for arg in args {
        match &arg {
            Object::String(s) => print!("{s}"),
            other => print!("{other}"),
        }
    }
    std::io::stdout().flush().ok();
    Object::Null
}

pub fn socket_read_all(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::Socket(socket)) = args.get(0) {
        match socket.read_all() {
            Ok(data) => Object::String(Rc::new(data)),
            Err(_) => Object::Null,
        }
    } else {
        Object::Null
    }
}

pub fn native_exit(args: Vec<Object>, _: &mut Interpreter) -> Object {
    let code = args.first().map(|a| a.to_f64() as i32).unwrap_or(0);
    std::process::exit(code);
}

pub fn native_chr(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::Number(n)) = args.first() {
        return Object::String(Rc::new(char::from(*n as u8).to_string()));
    }
    Object::Null
}

pub fn native_readline(_args: Vec<Object>, _: &mut Interpreter) -> Object {
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) => Object::Null,
        Ok(_) => Object::String(Rc::new(line.trim_end().to_string())),
        Err(_) => Object::Null,
    }
}

pub fn native_eval(args: Vec<Object>, vm: &mut Interpreter) -> Object {
    if let Some(Object::String(code)) = args.first() {
        return vm.eval(code);
    }
    Object::Null
}

pub fn get_var_from_str(args: Vec<Object>, vm: &mut Interpreter) -> Object {
    let Some(Object::String(name)) = args.get(0) else {
        return Object::Null;
    };
    let resolved = if matches!(args.get(1), Some(Object::Bool(true))) {
        vm.get_var_from_caller_scope(&intern(name))
    } else {
        vm.get_var(&intern(name)).map(|v| v.clone())
    };
    resolved.unwrap_or(Object::Null)
}

pub fn open_file(args: Vec<Object>, _: &mut Interpreter) -> Object {
    use std::fs::OpenOptions;

    if let Some(Object::String(filename)) = args.get(0) {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&**filename);
        match file {
            Ok(f) => Object::File(FileHandler::new(f, (**filename).clone())),
            Err(_) => Object::Null,
        }
    } else {
        Object::Null
    }
}

pub fn read_file(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::File(file)) = args.get(0) {
        let file_content = file.read();

        if let Some(content) = file_content.ok() {
            return Object::String(Rc::new(content));
        } else {
            return Object::Null;
        }
    }
    return Object::Null;
}

pub fn read_range(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::File(file)) = args.get(0) {
        if let Some(Object::Number(start)) = args.get(1) {
            if let Some(Object::Number(end)) = args.get(2) {
                let file_content = file.read_range(*start as usize, *end as usize);

                if let Some(content) = file_content.ok() {
                    let content_string = String::from_utf8_lossy(&content).to_string();
                    return Object::String(Rc::new(content_string));
                } else {
                    return Object::Null;
                }
            }
        }
    }
    return Object::Null;
}

pub fn write_file(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::File(file)) = args.get(0) {
        if let Some(Object::String(content)) = args.get(1) {
            let write_result = file.write(&content);

            if write_result.is_ok() {
                return Object::Bool(true);
            } else {
                return Object::Null;
            }
        }
    }
    return Object::Null;
}

pub fn write_file_range(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::File(file)) = args.get(0) {
        if let Some(Object::String(content)) = args.get(1) {
            if let Some(Object::Number(position)) = args.get(2) {
                let write_result = file.write_range(content.as_bytes(), *position as usize);

                if write_result.is_ok() {
                    return Object::Bool(true);
                } else {
                    return Object::Null;
                }
            }
        }
    }
    return Object::Null;
}

pub fn create_file(args: Vec<Object>, _: &mut Interpreter) -> Object {
    use std::fs::OpenOptions;

    if let Some(Object::String(filename)) = args.get(0) {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(&**filename);

        match file {
            Ok(f) => Object::File(FileHandler::new(f, (**filename).clone())),
            Err(_) => Object::Null,
        }
    } else {
        Object::Null
    }
}

pub fn list_dir(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::String(path)) = args.get(0) {
        let entries = std::fs::read_dir(&**path)
            .map(|dir| {
                dir.filter_map(|e| e.ok())
                    .map(|e| Object::String(Rc::new(e.file_name().to_string_lossy().to_string())))
                    .collect::<Vec<Object>>()
            })
            .ok();
        return entries.map(Object::List).unwrap_or(Object::Null);
    }
    Object::Null
}

pub fn exists(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::String(path)) = args.get(0) {
        return Object::Bool(std::path::Path::new(&**path).exists());
    }
    Object::Null
}

pub fn delete_file(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::String(path)) = args.get(0) {
        return Object::Bool(std::fs::remove_file(&**path).is_ok());
    }
    Object::Null
}

pub fn append_file(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let (Some(Object::String(path)), Some(Object::String(data))) = (args.get(0), args.get(1)) {
        use std::io::Write;
        let result = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&**path)
            .and_then(|mut f| f.write_all(data.as_bytes()));
        return Object::Bool(result.is_ok());
    }
    Object::Null
}

pub fn socket_connect(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let (Some(Object::String(address)), Some(Object::Number(port))) = (args.get(0), args.get(1))
    {
        match Socket::new_connect((**address).clone(), *port as u16) {
            Ok(socket) => Object::Socket(socket),
            Err(_) => Object::Null,
        }
    } else {
        Object::Null
    }
}

pub fn socket_bind(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let (Some(Object::String(address)), Some(Object::Number(port))) = (args.get(0), args.get(1))
    {
        match Socket::new_bind((**address).clone(), *port as u16) {
            Ok(socket) => Object::Socket(socket),
            Err(_) => Object::Null,
        }
    } else {
        Object::Null
    }
}

pub fn socket_accept(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::Socket(socket)) = args.get(0) {
        match socket.accept() {
            Ok(socket) => Object::Socket(socket),
            Err(_) => Object::Null,
        }
    } else {
        Object::Null
    }
}

pub fn socket_read(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::Socket(socket)) = args.get(0) {
        match socket.read() {
            Ok(data) => Object::String(Rc::new(data)),
            Err(_) => Object::Null,
        }
    } else {
        Object::Null
    }
}

pub fn socket_read_bytes(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let (Some(Object::Socket(socket)), Some(Object::Number(len))) = (args.get(0), args.get(1)) {
        match socket.read_bytes(*len as usize) {
            Ok(data) => {
                let data_string = String::from_utf8_lossy(&data).to_string();
                Object::String(Rc::new(data_string))
            }
            Err(_) => Object::Null,
        }
    } else {
        Object::Null
    }
}

pub fn socket_write(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let (Some(Object::Socket(socket)), Some(Object::String(data))) = (args.get(0), args.get(1)) {
        match socket.write(&data) {
            Ok(_) => Object::Bool(true),
            Err(_) => Object::Null,
        }
    } else {
        Object::Null
    }
}

pub fn socket_close(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::Socket(socket)) = args.get(0) {
        match socket.close() {
            Ok(_) => Object::Bool(true),
            Err(_) => Object::Null,
        }
    } else {
        Object::Null
    }
}

pub fn socket_is_connected(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::Socket(socket)) = args.get(0) {
        Object::Bool(socket.is_connected())
    } else {
        Object::Bool(false)
    }
}

pub fn socket_local_addr(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::Socket(socket)) = args.get(0) {
        match socket.local_addr() {
            Ok(addr) => Object::String(Rc::new(addr)),
            Err(_) => Object::Null,
        }
    } else {
        Object::Null
    }
}

pub fn socket_peer_addr(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::Socket(socket)) = args.get(0) {
        match socket.peer_addr() {
            Ok(addr) => Object::String(Rc::new(addr)),
            Err(_) => Object::Null,
        }
    } else {
        Object::Null
    }
}

pub fn dlopen(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::String(path)) = args.first() {
        match unsafe { Library::new(&**path) } {
            Ok(lib) => Object::Lib {
                path: (**path).clone(),
                lib: Rc::new(lib),
            },
            Err(_) => Object::Null,
        }
    } else {
        Object::Null
    }
}

pub fn dlsym(args: Vec<Object>, interp: &mut Interpreter) -> Object {
    if let (Some(Object::Lib { lib, .. }), Some(Object::String(name)), Some(Object::String(sig))) =
        (args.get(0), args.get(1), args.get(2))
    {
        let parsed = match ffi::parse_sig(sig) {
            Ok(p) => p,
            Err(e) => {
                Logger::error(&e, interp.current_loc, ErrorType::RunTime);
                return Object::Null;
            }
        };
        unsafe {
            let symbol: Symbol<*mut c_void> = match lib.get(name.as_bytes()) {
                Ok(s) => s,
                Err(_) => return Object::Null,
            };
            return Object::ForeignFn {
                symbol: *symbol,
                lib: lib.clone(),
                name: (**name).clone(),
                cif: ffi::build_cif(&parsed),
                sig: Rc::new(parsed),
            };
        }
    }
    Object::Null
}

pub fn def_struct(args: Vec<Object>, interp: &mut Interpreter) -> Object {
    if let (Some(Object::String(name)), Some(Object::String(spec))) = (args.get(0), args.get(1)) {
        let name_id = intern(&**name);
        let mut fields = vec![];
        for part in spec.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (fname, ftype) = match part.split_once(':') {
                Some((f, t)) => (f.trim(), t.trim()),
                None => {
                    Logger::error(
                        &format!("Bad field `{part}`, expected `name:type`"),
                        interp.current_loc,
                        ErrorType::RunTime,
                    );
                    return Object::Null;
                }
            };
            match ffi::parse_type(ftype) {
                Ok(t) => fields.push((intern(fname), t)),
                Err(e) => {
                    Logger::error(&e, interp.current_loc, ErrorType::RunTime);
                    return Object::Null;
                }
            }
        }
        let layout = ffi::compute_layout(name_id, fields);
        ffi::register_struct(name_id, layout);
        return Object::String(Rc::new((**name).clone()));
    }
    Object::Null
}

pub fn struct_val(args: Vec<Object>, interp: &mut Interpreter) -> Object {
    if let (Some(Object::String(name)), Some(Object::List(vals))) = (args.get(0), args.get(1)) {
        let layout = match ffi::get_struct(intern(&**name)) {
            Some(l) => l,
            None => {
                Logger::error(
                    &format!("Unknown struct `{name}`"),
                    interp.current_loc,
                    ErrorType::RunTime,
                );
                return Object::Null;
            }
        };
        if vals.len() != layout.fields.len() {
            Logger::error(
                &format!(
                    "struct `{name}` needs {} values, got {}",
                    layout.fields.len(),
                    vals.len()
                ),
                interp.current_loc,
                ErrorType::RunTime,
            );
            return Object::Null;
        }
        let mut bytes = vec![0u8; layout.size];
        let mut strings = vec![];
        for ((_, t, off), v) in layout.fields.iter().zip(vals) {
            if let Err(e) = ffi::marshal_scalar(t, v, bytes[*off..].as_mut_ptr(), &mut strings) {
                Logger::error(&e, interp.current_loc, ErrorType::RunTime);
                return Object::Null;
            }
        }
        return Object::CStruct {
            layout,
            bytes: Rc::new(bytes),
        };
    }
    Object::Null
}

pub fn get_field(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let (Some(obj), Some(Object::String(fname))) = (args.first(), args.get(1)) {
        return ffi::struct_field(obj, intern(&**fname));
    }
    Object::Null
}

pub fn set_field(args: Vec<Object>, interp: &mut Interpreter) -> Object {
    if let (Some(obj), Some(Object::String(fname)), Some(value)) =
        (args.first(), args.get(1), args.get(2))
    {
        return ffi::set_struct_field(obj, intern(&**fname), value, interp.current_loc);
    }
    Object::Null
}

pub fn byref(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::CStruct { bytes, .. }) = args.first() {
        return Object::Ptr(bytes.as_ptr() as *mut c_void);
    }
    Object::Null
}

pub fn null_ptr(_: Vec<Object>, _: &mut Interpreter) -> Object {
    Object::Ptr(std::ptr::null_mut())
}

// math
fn narg(args: &[Object], i: usize) -> Option<f64> {
    match args.get(i) {
        Some(Object::Number(n)) => Some(*n),
        _ => None,
    }
}
macro_rules! math1 {
    ($name:ident, $expr:expr) => {
        pub fn $name(args: Vec<Object>, _: &mut Interpreter) -> Object {
            if let Some(n) = narg(&args, 0) {
                Object::Number($expr(n))
            } else {
                Object::Null
            }
        }
    };
}
math1!(math_sin, f64::sin);
math1!(math_cos, f64::cos);
math1!(math_tan, f64::tan);
math1!(math_asin, f64::asin);
math1!(math_acos, f64::acos);
math1!(math_atan, f64::atan);
math1!(math_sqrt, f64::sqrt);
math1!(math_exp, f64::exp);
math1!(math_ln, f64::ln);
math1!(math_log10, f64::log10);
math1!(math_abs, f64::abs);
math1!(math_floor, f64::floor);
math1!(math_ceil, f64::ceil);
math1!(math_round, f64::round);
math1!(math_trunc, f64::trunc);
math1!(math_sinh, f64::sinh);
math1!(math_cosh, f64::cosh);
math1!(math_tanh, f64::tanh);

pub fn math_pow(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let (Some(a), Some(b)) = (narg(&args, 0), narg(&args, 1)) {
        Object::Number(a.powf(b))
    } else {
        Object::Null
    }
}
pub fn math_atan2(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let (Some(y), Some(x)) = (narg(&args, 0), narg(&args, 1)) {
        Object::Number(y.atan2(x))
    } else {
        Object::Null
    }
}
pub fn math_hypot(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let (Some(a), Some(b)) = (narg(&args, 0), narg(&args, 1)) {
        Object::Number(a.hypot(b))
    } else {
        Object::Null
    }
}
pub fn math_min(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let (Some(a), Some(b)) = (narg(&args, 0), narg(&args, 1)) {
        Object::Number(a.min(b))
    } else {
        Object::Null
    }
}
pub fn math_max(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let (Some(a), Some(b)) = (narg(&args, 0), narg(&args, 1)) {
        Object::Number(a.max(b))
    } else {
        Object::Null
    }
}
pub fn math_clamp(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let (Some(v), Some(lo), Some(hi)) = (narg(&args, 0), narg(&args, 1), narg(&args, 2)) {
        Object::Number(v.clamp(lo, hi))
    } else {
        Object::Null
    }
}
pub fn math_rand(args: Vec<Object>, _: &mut Interpreter) -> Object {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    if args.is_empty() {
        Object::Number(rng.gen::<f64>())
    } else if let (Some(lo), Some(hi)) = (narg(&args, 0), narg(&args, 1)) {
        if hi <= lo {
            Object::Number(lo)
        } else {
            Object::Number(rng.gen_range(lo..hi))
        }
    } else if let Some(hi) = narg(&args, 0) {
        Object::Number(rng.gen_range(0.0..hi))
    } else {
        Object::Null
    }
}
pub fn math_rand_range(args: Vec<Object>, interp: &mut Interpreter) -> Object {
    math_rand(args, interp)
}

// time
pub fn time_now(args: Vec<Object>, _: &mut Interpreter) -> Object {
    let _ = args;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    Object::Number(secs)
}
pub fn time_millis(args: Vec<Object>, _: &mut Interpreter) -> Object {
    let _ = args;
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);
    Object::Number(ms)
}
pub fn time_sleep(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(ms) = narg(&args, 0) {
        std::thread::sleep(std::time::Duration::from_millis(ms.max(0.0) as u64));
        Object::Bool(true)
    } else {
        Object::Null
    }
}

// env / process
pub fn env_args(_args: Vec<Object>, vm: &mut Interpreter) -> Object {
    Object::List(
        vm.script_args
            .iter()
            .map(|s| Object::String(Rc::new(s.clone())))
            .collect(),
    )
}
pub fn env_raw_args(args: Vec<Object>, _: &mut Interpreter) -> Object {
    let _ = args;
    Object::List(
        std::env::args()
            .map(|s| Object::String(Rc::new(s)))
            .collect(),
    )
}
pub fn env_var(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::String(name)) = args.get(0) {
        std::env::var(&**name)
            .map(|v| Object::String(Rc::new(v)))
            .unwrap_or(Object::Null)
    } else {
        Object::Null
    }
}
pub fn env_cwd(args: Vec<Object>, _: &mut Interpreter) -> Object {
    let _ = args;
    std::env::current_dir()
        .map(|p| Object::String(Rc::new(p.to_string_lossy().into_owned())))
        .unwrap_or(Object::Null)
}
pub fn env_set_var(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let (Some(Object::String(k)), Some(Object::String(v))) = (args.get(0), args.get(1)) {
        // SAFETY: single-threaded interpreter, no concurrent env access
        unsafe { std::env::set_var(&**k, &**v) };
        Object::Bool(true)
    } else {
        Object::Null
    }
}
