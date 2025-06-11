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
