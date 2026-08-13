use crate::file_handler::FileHandler;
use crate::interner::intern;
use crate::interpreter::Interpreter;
use crate::object::Object;
use crate::socket::Socket;
use std::rc::Rc;

pub type NativeFn = fn(Vec<Object>, &mut Interpreter) -> Object;

pub fn native_write(args: Vec<Object>, _: &mut Interpreter) -> Object {
    for arg in args {
        print!("{}", arg);
    }
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

pub fn get_var_from_str(args: Vec<Object>, vm: &mut Interpreter) -> Object {
    if let Some(Object::String(name)) = args.get(0) {
        return vm.get_var(&intern(name)).unwrap_or(&mut Object::Null).clone();
    }
    Object::Null
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
