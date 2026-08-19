# IOk Programming Language
![IOk Logo](./img/logo.png)

## **An Interpreted Language Built in Rust**
*Work in Progress - Contributions Welcome!*

IOk is a modern interpreted language focused on simplicity and performance. Built entirely in Rust, it combines Rust's safety and expressive syntax.

```rust
// Basic syntax examples
import std::io::print
let name = "IOk"
print("Hello, {name}!")  // Hello, IOk!

// Functions with multiple return styles
fn add(a, b) => a + b
fn div(a, b) {
    if b == 0 {
        ret null
    }
    ret a/b
}

// Collections and iteration
let nums = [1, 2, 3, 4]
for num -> nums {
    write(num * 2)  // 2, 4, 6, 8
}

// Structs and methods
struct Point {
    let x = 0
    let y = 0
    
    fn move(dx, dy) => {
        self.x += dx
        self.y += dy
    }
}

let p = Point { x: 5, y: 10 }
p.move(3, -2)
```
## Networking
```rust
// Networking via std/net.iok
import "net.iok" @ net

fn server() {
    let srv = net::bind("127.0.0.1", 8080)
    let client = srv.accept()
    write(client.read_all())
}

fn client() {
    let c = net::connect("127.0.0.1", 8080)
    c.write("Hello, server!")
    c.close()
}

// One-line HTTP GET
let html = net::http_get("example.com", 80, "/")
```
## Raylib Bindings
```rust
// Full raylib 6.0 bindings via FFI (extern/raylib_6.0/lib/raylib.iok)
// Requires the raylib 6.0 shared library (.dll / .so) on your PATH
import "../extern/raylib_6.0/lib/raylib.iok" @ rl

rl::init_window(800, 450, "IOk Pong")
rl::set_target_fps(60)

let player = Paddle::new(20, 180, 6, -1)   // W/S to move, Tab for bot-vs-bot
let bot = Paddle::new(768, 180, 4.5, 1)

while !rl::window_should_close() {
    player.clamp()
    bot.ai_move(ball)
    rl::begin_drawing()
    rl::clear_background(rl::color(20, 20, 30))
    rl::draw_rectangle_rec(player.rec, rl::color(80, 200, 120))
    rl::draw_circle_v(ball.pos, ball.r, rl::color(255, 80, 80))
    rl::end_drawing()
}

rl::close_window()
```
See `examples/raylib.iok` for the full pong game (first to 5 wins).
## TODO
Task  | Implemented
------------- | -------------
Lists | ✅
Functions | ✅
Struct |  ✅
Imports |  ✅
STD Lib | 🚧 Work in progress
File IO | ⚠ Basic support for now
Socket / Network | ✅
FFI | ✅
Optimize | ⚠ I think it's fast enough for a tree-walk interprter
Bytecode | 🚧 Not yet, but will be implemented

## Getting Started

### Prerequisites

-   Rust 1.60+ (install via [rustup](https://rustup.rs/))
    

### Installation
```bash
git clone https://github.com/KirolosBoshra/iok.git
cd iok
cargo build --release
```
### Running Programs
```bash
# Run file
./target/release/iok --std ./std/ ./examples/hello.iok
# Run webserver example
./target/release/iok --std ./std/ ./examples/webserver/server.iok
# Run TODO web app example (open http://127.0.0.1:8081/)
./target/release/iok --std ./std/ ./examples/todo_web/server.iok
# Or copy std dir to target/release/ and just run
./target/release/iok ./examples/hello.iok
# Start Interpreter
./target/release/iok --std ./std/
```
