use crate::file_handler::FileHandler;
use crate::interpreter::Interpreter;
use crate::object::Object;

pub type NativeFn = fn(Vec<Object>, &mut Interpreter) -> Object;

pub fn native_write(args: Vec<Object>, _: &mut Interpreter) -> Object {
    for arg in args {
        print!("{}", arg);
    }
    Object::Null
}

pub fn native_exit(args: Vec<Object>, _: &mut Interpreter) -> Object {
    if let Some(Object::Number(code)) = args.get(0) {
        std::process::exit(*code as i32);
    } else {
        std::process::exit(-1);
    }
}

pub fn get_var_from_str(args: Vec<Object>, vm: &mut Interpreter) -> Object {
    if let Some(Object::String(name)) = args.get(0) {
        return vm.get_var(name).unwrap_or(&mut Object::Null).clone();
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
            return Object::String(Box::new(content));
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
                    return Object::String(Box::new(content_string));
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
