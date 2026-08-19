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

pub fn native_exit(_args: Vec<Object>, _: &mut Interpreter) -> Object {
    std::process::exit(0);
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
        Object::Socket(Socket::new_connect((**address).clone(), *port as u16))
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
                sig: Rc::new(parsed),
            };
        }
    }
    Object::Null
}

pub fn def_struct(args: Vec<Object>, interp: &mut Interpreter) -> Object {
    if let (Some(Object::String(name)), Some(Object::String(spec))) =
        (args.get(0), args.get(1))
    {
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
    if let (Some(Object::String(name)), Some(Object::List(vals))) =
        (args.get(0), args.get(1))
    {
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
