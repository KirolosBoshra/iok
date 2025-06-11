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
## TODO
Task  | Implemented
------------- | -------------
Lists | ✅
Functions | ✅
Struct |  ✅
Imports |  ✅
STD Lib | 🚧 Work in progress
File IO | ❌ Planned
Optimize | ❌ Not Planned

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
./target/release/iok examples/hello.iok
# Start Interprter
./target/release/iok
```
