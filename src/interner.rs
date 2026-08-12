use lazy_static::lazy_static;
use rustc_hash::FxHashMap;
use std::sync::Mutex;

lazy_static! {
    static ref TABLE: Mutex<InternTable> = Mutex::new(InternTable::default());
}

#[derive(Default)]
struct InternTable {
    map: FxHashMap<String, u32>,
    names: Vec<String>,
}

// Global interner, single-threaded interpreter so the Mutex only
// contends during parsing and symbol resolution, never in the hot loop
pub fn intern(s: &str) -> u32 {
    let mut t = TABLE.lock().unwrap();
    if let Some(&id) = t.map.get(s) {
        return id;
    }
    let id = t.names.len() as u32;
    t.names.push(s.to_string());
    t.map.insert(s.to_string(), id);
    id
}

pub fn resolve(id: u32) -> String {
    TABLE.lock().unwrap().names[id as usize].clone()
}
