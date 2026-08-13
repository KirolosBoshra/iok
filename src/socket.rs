use std::fmt;
use std::io::BufReader;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct SocketHandler {
    pub id: usize,
    pub address: String,
    pub port: u16,
    pub stream: Arc<Mutex<BufReader<TcpStream>>>,
}

impl PartialEq for SocketHandler {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Clone, Debug)]
pub struct SocketListener {
    pub id: usize,
    pub address: String,
    pub port: u16,
    pub listener: Arc<Mutex<TcpListener>>,
}

impl PartialEq for SocketListener {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Display for SocketListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Listener<{}:{}>", self.address, self.port)
    }
}

impl SocketListener {
    pub fn new(address: String, port: u16) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(format!("{}:{}", address, port))?;
        let arc_listener = Arc::new(Mutex::new(listener));
        let id = arc_listener.as_ref() as *const Mutex<TcpListener> as usize;

        Ok(SocketListener {
            id,
            address,
            port,
            listener: arc_listener,
        })
    }

    pub fn accept(&self) -> Result<SocketHandler, std::io::Error> {
        let (stream, _) = self.listener.lock().unwrap().accept()?;
        Ok(SocketHandler::from_stream(stream))
    }

    pub fn close(&self) -> Result<(), std::io::Error> {
        // TcpListener doesn't have an explicit close method in Rust
        // When the SocketListener is dropped, the underlying listener will be closed
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Socket {
    Handler(SocketHandler),
    Listener(SocketListener),
}

fn invalid_socket(msg: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, msg)
}

impl fmt::Display for Socket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Socket::Handler(h) => write!(f, "Socket<{}:{}>", h.address, h.port),
            Socket::Listener(l) => write!(f, "{}", l),
        }
    }
}

impl Socket {
    pub fn new_connect(address: String, port: u16) -> Socket {
        Socket::Handler(SocketHandler::new(address, port))
    }

    pub fn new_bind(address: String, port: u16) -> Result<Socket, std::io::Error> {
        Ok(Socket::Listener(SocketListener::new(address, port)?))
    }

    pub fn accept(&self) -> Result<Socket, std::io::Error> {
        match self {
            Socket::Listener(l) => l.accept().map(Socket::Handler),
            _ => Err(invalid_socket("accept requires a bound listener")),
        }
    }

    pub fn read(&self) -> Result<String, std::io::Error> {
        match self {
            Socket::Handler(h) => h.read(),
            _ => Err(invalid_socket("read requires a connected socket")),
        }
    }

    pub fn read_all(&self) -> Result<String, std::io::Error> {
        match self {
            Socket::Handler(h) => h.read_all(),
            _ => Err(invalid_socket("read_all requires a connected socket")),
        }
    }

    pub fn read_bytes(&self, len: usize) -> Result<Vec<u8>, std::io::Error> {
        match self {
            Socket::Handler(h) => h.read_bytes(len),
            _ => Err(invalid_socket("read_bytes requires a connected socket")),
        }
    }

    pub fn write(&self, data: &str) -> Result<(), std::io::Error> {
        match self {
            Socket::Handler(h) => h.write(data),
            _ => Err(invalid_socket("write requires a connected socket")),
        }
    }

    pub fn close(&self) -> Result<(), std::io::Error> {
        match self {
            Socket::Handler(h) => h.close(),
            Socket::Listener(l) => l.close(),
        }
    }

    pub fn is_connected(&self) -> bool {
        match self {
            Socket::Handler(h) => h.is_connected(),
            _ => false,
        }
    }

    pub fn local_addr(&self) -> Result<String, std::io::Error> {
        match self {
            Socket::Handler(h) => h.local_addr(),
            _ => Err(invalid_socket("local_addr requires a connected socket")),
        }
    }

    pub fn peer_addr(&self) -> Result<String, std::io::Error> {
        match self {
            Socket::Handler(h) => h.peer_addr(),
            _ => Err(invalid_socket("peer_addr requires a connected socket")),
        }
    }
}

impl SocketHandler {
    pub fn new(address: String, port: u16) -> Self {
        let stream = TcpStream::connect(format!("{}:{}", address, port)).unwrap();
        stream.set_read_timeout(Some(Duration::from_millis(5000))).ok();
        let arc_stream = Arc::new(Mutex::new(BufReader::new(stream)));
        let id = arc_stream.as_ref() as *const Mutex<BufReader<TcpStream>> as usize;
        SocketHandler {
            address,
            port,
            stream: arc_stream,
            id,
        }
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        stream.set_read_timeout(Some(Duration::from_millis(5000))).ok();
        let arc_stream = Arc::new(Mutex::new(BufReader::new(stream)));
        let id = arc_stream.as_ref() as *const Mutex<BufReader<TcpStream>> as usize;
        SocketHandler {
            address: "unknown".to_string(),
            port: 0,
            stream: arc_stream,
            id,
        }
    }

    pub fn read(&self) -> Result<String, std::io::Error> {
        use std::io::BufRead;

        let mut reader = self.stream.lock().unwrap();

        let mut buffer = String::new();

        match reader.read_line(&mut buffer) {
            Ok(0) => Ok(String::new()), // EOF
            Ok(_) => {
                // Remove trailing newline if present
                if buffer.ends_with('\n') {
                    buffer.pop();
                    if buffer.ends_with('\r') {
                        buffer.pop();
                    }
                }
                Ok(buffer)
            }
            Err(e) => {
                // Handle timeout gracefully
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock
                {
                    Ok(String::new()) // Return empty string on timeout
                } else {
                    Err(e)
                }
            }
        }
    }

    pub fn read_all(&self) -> Result<String, std::io::Error> {
        use std::io::Read;

        let mut reader = self.stream.lock().unwrap();

        let mut buffer = String::new();

        // Read all data until EOF
        match reader.read_to_string(&mut buffer) {
            Ok(0) => Ok(String::new()), // EOF
            Ok(_) => Ok(buffer),
            Err(e) => {
                // Handle timeout gracefully
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock
                {
                    Ok(buffer) // Return what we have so far
                } else {
                    Err(e)
                }
            }
        }
    }

    pub fn read_bytes(&self, len: usize) -> Result<Vec<u8>, std::io::Error> {
        use std::io::Read;

        let mut reader = self.stream.lock().unwrap();
        let mut buffer = vec![0u8; len];

        match reader.read_exact(&mut buffer) {
            Ok(_) => Ok(buffer),
            Err(e) => Err(e),
        }
    }

    pub fn write(&self, data: &str) -> Result<(), std::io::Error> {
        use std::io::Write;

        let mut reader = self.stream.lock().unwrap();
        reader.get_mut().write_all(data.as_bytes())
    }

    pub fn close(&self) -> Result<(), std::io::Error> {
        use std::net::Shutdown;

        let mut reader = self.stream.lock().unwrap();
        reader.get_mut().shutdown(Shutdown::Both)
    }

    pub fn is_connected(&self) -> bool {
        self.stream.lock().unwrap().get_ref().peer_addr().is_ok()
    }

    pub fn local_addr(&self) -> Result<String, std::io::Error> {
        let reader = self.stream.lock().unwrap();
        match reader.get_ref().local_addr() {
            Ok(addr) => Ok(addr.to_string()),
            Err(e) => Err(e),
        }
    }

    pub fn peer_addr(&self) -> Result<String, std::io::Error> {
        let reader = self.stream.lock().unwrap();
        match reader.get_ref().peer_addr() {
            Ok(addr) => Ok(addr.to_string()),
            Err(e) => Err(e),
        }
    }
}
