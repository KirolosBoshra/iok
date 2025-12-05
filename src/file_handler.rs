use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct SharedFile(pub Arc<Mutex<fs::File>>);

impl PartialEq for SharedFile {
    fn eq(&self, other: &Self) -> bool {
        // true if both Arcs point to the same allocation
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FileHandler {
    pub file: SharedFile,
    pub path: String,
}
impl FileHandler {
    pub fn new(file: fs::File, path: String) -> Self {
        FileHandler {
            file: SharedFile(Arc::new(Mutex::new(file))),
            path,
        }
    }
    pub fn write(&self, data: &str) -> Result<(), std::io::Error> {
        // Lock the file before writing (thread safe)
        let mut file = self.file.0.lock().unwrap();
        file.write_all(data.as_bytes())?;
        file.flush()
    }
    pub fn write_range(&self, data: &[u8], start: usize) -> Result<(), std::io::Error> {
        let mut file = self.file.0.lock().unwrap();
        file.seek(SeekFrom::Start(start as u64))?;
        file.write_all(data)?;
        file.flush()
    }
    pub fn read(&self) -> Result<String, std::io::Error> {
        let mut file = self.file.0.lock().unwrap();
        file.seek(SeekFrom::Start(0))?; // reset position to start
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }
    pub fn read_range(&self, start: usize, len: usize) -> Result<Vec<u8>, std::io::Error> {
        let mut file = self.file.0.lock().unwrap();
        file.seek(SeekFrom::Start(start as u64))?;
        let mut buffer = vec![0u8; len];
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);
        Ok(buffer)
    }
}
