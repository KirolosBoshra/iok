use crate::interner::intern;
use crate::logger::{ErrorType, Logger};
use crate::object::Object;
use lazy_static::lazy_static;
use libffi::middle::{Cif, Type};
use libffi::raw::ffi_call;
use rustc_hash::FxHashMap;
use std::alloc::{alloc, alloc_zeroed, dealloc, Layout};
use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_void;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref STRUCTS: Mutex<FxHashMap<u32, Arc<CLayout>>> = Mutex::new(FxHashMap::default());
}

#[derive(Clone, Debug)]
pub enum FType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Str,
    Ptr,
    Struct(Arc<CLayout>),
    StructPtr(Arc<CLayout>),
}

#[derive(Clone, Debug)]
pub struct CLayout {
    pub name: u32,
    pub fields: Vec<(u32, FType, usize)>,
    pub size: usize,
    pub align: usize,
}

#[derive(Clone, Debug)]
pub struct ParsedSig {
    pub args: Vec<FType>,
    pub ret: Option<FType>,
}

fn align_up(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

fn size_of(t: &FType) -> usize {
    match t {
        FType::I8 | FType::U8 => 1,
        FType::I16 | FType::U16 => 2,
        FType::I32 | FType::U32 | FType::F32 => 4,
        FType::I64 | FType::U64 | FType::F64 => 8,
        FType::Str | FType::Ptr | FType::StructPtr(_) => std::mem::size_of::<*const c_void>(),
        FType::Struct(l) => l.size,
    }
}

fn align_of(t: &FType) -> usize {
    match t {
        FType::I8 | FType::U8 => 1,
        FType::I16 | FType::U16 => 2,
        FType::I32 | FType::U32 | FType::F32 => 4,
        FType::I64 | FType::U64 | FType::F64 => 8,
        FType::Str | FType::Ptr | FType::StructPtr(_) => std::mem::size_of::<*const c_void>(),
        FType::Struct(l) => l.align,
    }
}

pub fn compute_layout(name: u32, fields: Vec<(u32, FType)>) -> CLayout {
    let mut size = 0usize;
    let mut align = 1usize;
    let mut out = vec![];
    for (fname, t) in fields {
        let a = align_of(&t);
        align = align.max(a);
        size = align_up(size, a);
        let tsize = size_of(&t);
        out.push((fname, t, size));
        size += tsize;
    }
    let size = align_up(size, align);
    CLayout {
        name,
        fields: out,
        size,
        align,
    }
}

pub fn register_struct(name: u32, layout: CLayout) {
    STRUCTS.lock().unwrap().insert(name, Arc::new(layout));
}

pub fn get_struct(name: u32) -> Option<Arc<CLayout>> {
    STRUCTS.lock().unwrap().get(&name).cloned()
}

pub fn parse_type(token: &str) -> Result<FType, String> {
    match token {
        "i8" => Ok(FType::I8),
        "i16" => Ok(FType::I16),
        "i32" => Ok(FType::I32),
        "i64" => Ok(FType::I64),
        "u8" => Ok(FType::U8),
        "u16" => Ok(FType::U16),
        "u32" => Ok(FType::U32),
        "u64" => Ok(FType::U64),
        "f32" => Ok(FType::F32),
        "f64" => Ok(FType::F64),
        "str" => Ok(FType::Str),
        "ptr" => Ok(FType::Ptr),
        t if t.starts_with('*') => {
            let name = intern(&t[1..]);
            get_struct(name)
                .map(FType::StructPtr)
                .ok_or_else(|| format!("Unknown struct `{}`", &t[1..]))
        }
        t => {
            let name = intern(t);
            get_struct(name)
                .map(FType::Struct)
                .ok_or_else(|| format!("Unknown type `{t}`"))
        }
    }
}

pub fn parse_sig(sig: &str) -> Result<ParsedSig, String> {
    let (arg_part, ret_part) = match sig.split_once("->") {
        Some((a, r)) => (a, Some(r.trim())),
        None => (sig, None),
    };
    let mut args = vec![];
    for tok in arg_part.split(',') {
        let tok = tok.trim();
        if tok.is_empty() {
            continue;
        }
        args.push(parse_type(tok)?);
    }
    let ret = match ret_part {
        None | Some("void") => None,
        Some(t) => Some(parse_type(t)?),
    };
    Ok(ParsedSig { args, ret })
}

fn to_ffi_type(t: &FType) -> Type {
    match t {
        FType::I8 => Type::i8(),
        FType::I16 => Type::i16(),
        FType::I32 => Type::i32(),
        FType::I64 => Type::i64(),
        FType::U8 => Type::u8(),
        FType::U16 => Type::u16(),
        FType::U32 => Type::u32(),
        FType::U64 => Type::u64(),
        FType::F32 => Type::f32(),
        FType::F64 => Type::f64(),
        FType::Str | FType::Ptr | FType::StructPtr(_) => Type::pointer(),
        FType::Struct(l) => Type::structure(l.fields.iter().map(|(_, t, _)| to_ffi_type(t))),
    }
}

// 16-aligned scratch cells for scalar/pointer args and returns
#[repr(C, align(16))]
struct Cell([u8; 64]);

struct AlignedBuf {
    ptr: *mut u8,
    layout: Layout,
}

impl AlignedBuf {
    fn new(size: usize, align: usize) -> Self {
        let layout = Layout::from_size_align(size.max(1), align).expect("bad layout");
        let ptr = unsafe { alloc(layout) };
        AlignedBuf { ptr, layout }
    }

    fn zeroed(size: usize, align: usize) -> Self {
        let layout = Layout::from_size_align(size.max(1), align).expect("bad layout");
        let ptr = unsafe { alloc_zeroed(layout) };
        AlignedBuf { ptr, layout }
    }

    fn as_ptr(&self) -> *mut c_void {
        self.ptr as *mut c_void
    }

    fn copy_from(&mut self, bytes: &[u8]) {
        let dst = unsafe { std::slice::from_raw_parts_mut(self.ptr, self.layout.size()) };
        dst[..bytes.len()].copy_from_slice(bytes);
    }

    unsafe fn bytes(&self) -> &[u8] {
        std::slice::from_raw_parts(self.ptr, self.layout.size())
    }
}

impl Drop for AlignedBuf {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr, self.layout) }
    }
}

macro_rules! write_num {
    ($v:ident, $t:ty, $f:expr, $o:expr, $d:expr) => {
        if matches!($f, FType::$v) {
            match $o {
                Object::Number(n) => {
                    unsafe { *($d as *mut $t) = *n as $t }
                    return Ok(());
                }
                _ => return Err("expected Number".into()),
            }
        }
    };
}

macro_rules! read_num {
    ($v:ident, $t:ty, $f:expr, $b:expr, $o:expr) => {
        if matches!($f, FType::$v) {
            return Object::Number(unsafe { *($b.as_ptr().add($o) as *const $t) } as f64);
        }
    };
}

pub fn marshal_scalar(
    t: &FType,
    obj: &Object,
    dst: *mut u8,
    strings: &mut Vec<CString>,
) -> Result<(), String> {
    write_num!(I8, i8, t, obj, dst);
    write_num!(I16, i16, t, obj, dst);
    write_num!(I32, i32, t, obj, dst);
    write_num!(I64, i64, t, obj, dst);
    write_num!(U8, u8, t, obj, dst);
    write_num!(U16, u16, t, obj, dst);
    write_num!(U32, u32, t, obj, dst);
    write_num!(U64, u64, t, obj, dst);
    write_num!(F32, f32, t, obj, dst);
    write_num!(F64, f64, t, obj, dst);
    match t {
        FType::Str => match obj {
            Object::String(s) => {
                let cs = CString::new(s.as_str()).map_err(|_| "string contains NUL".to_string())?;
                unsafe { *(dst as *mut *const c_char) = cs.as_ptr() };
                strings.push(cs);
            }
            Object::Ptr(p) => unsafe { *(dst as *mut *const c_char) = (*p) as *const c_char },
            _ => return Err("expected String".into()),
        },
        FType::Ptr => match obj {
            Object::Ptr(p) => unsafe { *(dst as *mut *mut c_void) = *p },
            _ => return Err("expected Ptr".into()),
        },
        _ => return Err("unsupported type".into()),
    }
    Ok(())
}

// struct fields are numeric/nested only char* fields need the CString
// to outlive the struct bytes, add an owned-strings list to CStruct when needed.
fn read_field(bytes: &[u8], t: &FType, off: usize) -> Object {
    read_num!(I8, i8, t, bytes, off);
    read_num!(I16, i16, t, bytes, off);
    read_num!(I32, i32, t, bytes, off);
    read_num!(I64, i64, t, bytes, off);
    read_num!(U8, u8, t, bytes, off);
    read_num!(U16, u16, t, bytes, off);
    read_num!(U32, u32, t, bytes, off);
    read_num!(U64, u64, t, bytes, off);
    read_num!(F32, f32, t, bytes, off);
    read_num!(F64, f64, t, bytes, off);
    match t {
        FType::Struct(l) => Object::CStruct {
            layout: l.clone(),
            bytes: Rc::new(bytes[off..off + l.size].to_vec()),
        },
        _ => Object::Null,
    }
}

fn write_field(bytes: &mut [u8], t: &FType, off: usize, obj: &Object) -> Result<(), String> {
    let mut strings = vec![];
    marshal_scalar(t, obj, bytes[off..].as_mut_ptr(), &mut strings)?;
    Ok(())
}

pub fn struct_field(obj: &Object, fname: u32) -> Object {
    if let Object::CStruct { layout, bytes } = obj {
        for (name, t, off) in &layout.fields {
            if *name == fname {
                return read_field(bytes, t, *off);
            }
        }
    }
    Object::Null
}

pub fn set_struct_field(obj: &Object, fname: u32, value: &Object) -> Object {
    if let Object::CStruct { layout, bytes } = obj {
        for (name, t, off) in &layout.fields {
            if *name == fname {
                let mut bytes = (**bytes).clone();
                match write_field(&mut bytes, t, *off, value) {
                    Ok(()) => {
                        return Object::CStruct {
                            layout: layout.clone(),
                            bytes: Rc::new(bytes),
                        }
                    }
                    Err(e) => {
                        Logger::error(&format!("set_field: {e}"), None, ErrorType::RunTime);
                        return Object::Null;
                    }
                }
            }
        }
    }
    Object::Null
}

fn check_struct_size(bytes: &[u8], l: &CLayout) -> bool {
    if bytes.len() != l.size {
        Logger::error(
            &format!(
                "struct size mismatch: expected {}, got {}",
                l.size,
                bytes.len()
            ),
            None,
            ErrorType::RunTime,
        );
        return false;
    }
    true
}

pub fn call_foreign(sig: &ParsedSig, symbol: *mut c_void, args: &[Object]) -> Object {
    if args.len() != sig.args.len() {
        Logger::error(
            &format!("Expected {} args, got {}", sig.args.len(), args.len()),
            None,
            ErrorType::RunTime,
        );
        return Object::Null;
    }

    let cif = Cif::new(
        sig.args.iter().map(to_ffi_type).collect::<Vec<_>>(),
        match &sig.ret {
            Some(t) => to_ffi_type(t),
            None => Type::void(),
        },
    );

    let mut cells: Vec<Box<Cell>> = vec![];
    let mut strings: Vec<CString> = vec![];
    let mut bufs: Vec<AlignedBuf> = vec![];
    let mut avalue: Vec<*mut c_void> = vec![];

    for (t, a) in sig.args.iter().zip(args) {
        match t {
            FType::Struct(l) => {
                if let Object::CStruct { bytes, .. } = a {
                    if !check_struct_size(bytes, l) {
                        return Object::Null;
                    }
                    let mut buf = AlignedBuf::new(l.size, l.align);
                    buf.copy_from(bytes);
                    avalue.push(buf.as_ptr());
                    bufs.push(buf);
                } else if let Object::Instance { fields, .. } = a {
                    // pass a language struct: marshaled field-by-field into the C struct
                    let buf = AlignedBuf::zeroed(l.size, l.align);
                    for (fname, t, off) in &l.fields {
                        let val = fields.get(fname);
                        match val {
                            Some(v) => {
                                let mut strings = vec![];
                                let dst = unsafe { buf.ptr.add(*off) };
                                if let Err(e) = marshal_scalar(t, v, dst, &mut strings) {
                                    Logger::error(
                                        &format!("bad struct field: {e}"),
                                        None,
                                        ErrorType::RunTime,
                                    );
                                    return Object::Null;
                                }
                            }
                            None => {
                                Logger::error(
                                    "struct argument missing a required field",
                                    None,
                                    ErrorType::RunTime,
                                );
                                return Object::Null;
                            }
                        }
                    }
                    avalue.push(buf.as_ptr());
                    bufs.push(buf);
                } else {
                    Logger::error(
                        "expected CStruct or Instance for struct argument",
                        None,
                        ErrorType::RunTime,
                    );
                    return Object::Null;
                }
            }
            FType::StructPtr(l) => {
                let mut cell = Box::new(Cell([0u8; 64]));
                let ptr = cell.0.as_mut_ptr();
                match a {
                    Object::CStruct { bytes, .. } => {
                        if !check_struct_size(bytes, l) {
                            return Object::Null;
                        }
                        unsafe { *(ptr as *mut *const u8) = bytes.as_ptr() };
                    }
                    Object::Ptr(p) => unsafe { *(ptr as *mut *mut c_void) = *p },
                    _ => {
                        Logger::error(
                            "expected CStruct or Ptr for struct pointer argument",
                            None,
                            ErrorType::RunTime,
                        );
                        return Object::Null;
                    }
                }
                avalue.push(ptr as *mut c_void);
                cells.push(cell);
            }
            _ => {
                let mut cell = Box::new(Cell([0u8; 64]));
                let ptr = cell.0.as_mut_ptr();
                if let Err(e) = marshal_scalar(t, a, ptr, &mut strings) {
                    Logger::error(&format!("bad argument: {e}"), None, ErrorType::RunTime);
                    return Object::Null;
                }
                avalue.push(ptr as *mut c_void);
                cells.push(cell);
            }
        }
    }

    let mut ret_cell = Box::new(Cell([0u8; 64]));
    let mut ret_buf: Option<AlignedBuf> = None;
    let rvalue: *mut c_void = match &sig.ret {
        Some(FType::Struct(l)) => {
            let b = AlignedBuf::new(l.size, l.align);
            let p = b.as_ptr();
            ret_buf = Some(b);
            p
        }
        Some(_) => ret_cell.0.as_mut_ptr() as *mut c_void,
        None => std::ptr::null_mut(),
    };

    unsafe {
        ffi_call(
            cif.as_raw_ptr(),
            Some(std::mem::transmute::<*mut c_void, unsafe extern "C" fn()>(
                symbol,
            )),
            rvalue,
            avalue.as_mut_ptr(),
        );
    }

    // numeric returns land in the zeroed cell; reading exact width from its
    // start equals the full-register read (same low bytes)
    match &sig.ret {
        None => Object::Null,
        Some(FType::Str) => {
            let p = unsafe { *(rvalue as *const *const c_char) };
            if p.is_null() {
                Object::Null
            } else {
                Object::String(Rc::new(
                    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned(),
                ))
            }
        }
        Some(FType::Ptr) => Object::Ptr(unsafe { *(rvalue as *const *mut c_void) }),
        Some(FType::Struct(l)) => {
            let b = ret_buf.unwrap();
            Object::CStruct {
                layout: l.clone(),
                bytes: Rc::new(unsafe { b.bytes() }.to_vec()),
            }
        }
        Some(FType::StructPtr(_)) => Object::Null,
        Some(t) => read_field(&ret_cell.0, t, 0),
    }
}

