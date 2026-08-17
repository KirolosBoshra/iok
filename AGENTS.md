# AGENTS.md

Tree-walk interpreter for the IOk language, written in Rust. Single binary crate (no lib.rs), no tests, no CI, no lint config.

## Build & verify

- `cargo build` / `cargo build --release` (release profile uses `lto = "thin"`, `codegen-units = 1`)
- There are **no tests**. Verification is running examples: `cargo run -- --std ./std/ ./examples/<name>.iok`
- `examples/` is the de-facto test suite; each one exercises a feature (sockets, imports, structs, match, webserver). Run the relevant one after touching the interpreter/parser.

## Architecture

- Pipeline: `Lexer` -> `Parser` (precedence-climbing, see commit `f3024cb`) -> `Interpreter` (tree-walk over `Object`s).
- Modules: `lexer.rs`, `parser.rs`, `interpreter.rs`, `object.rs`, `interner.rs` (string interning via `rustc-hash`), `socket.rs`, `file_handler.rs`, `std_native.rs`, `logger.rs`, `ffi.rs`.
- FFI (`ffi.rs`, `std/ffi.iok`, `examples/ffi.iok`): ctypes-style dynamic calls via `libloading` + `libffi`. `ffi::load(path)` -> `Lib`, `ffi::sym(lib, name, sig)` -> `ForeignFn`; sig grammar `"i32,i64,f64,str,ptr,Point,*Point -> i32"` (struct names registered via `ffi::def_struct(name, "x:i32, y:i32")`). Structs are raw byte buffers (`Object::CStruct`); pass by pointer with `ffi::byref(v)`; struct-by-value args and struct returns supported; `str` fields inside structs are NOT (only numeric/nested). Wrong signature = UB, same as Python ctypes.
- The stdlib (`std/*.iok`) is **written in IOk itself**. `import std::io::print` resolves against the `--std` dir; `import "net.iok" @ net` resolves relative to the importing file. Example: `examples/import/`.
- Native functions (Rust) are registered in `Interpreter::new` (src/interpreter.rs:60-111) under double-underscore names (`__open_file`, `__socket_bind`, ...); the .iok stdlib wraps them with public names. Adding a native: implement in `std_native.rs`, register in `Interpreter::new`, wrap in `std/*.iok`.
- If `--std` is omitted, the interpreter falls back to `std/` next to the executable — so bare `./target/release/iok example.iok` only works if you copied the std dir there. Always pass `--std ./std/`.
- Errors: `Logger` accumulates errors across statements; `main.rs` exits 1 if any were logged. Don't use `panic!`/`eprintln!` for user-facing errors — go through `Logger`.

## IOk language syntax (from `examples/`)

- `let x = val`, `let x` (uninitialized), `null`. Reassignment: `x = val`.
- Functions: `fn add(a, b=1) => expr` (expression body) or `fn div(a, b) => { ... }` (block body). **`ret`** returns, **not** `return`.
- Anonymous functions: `let shout = (s) => { ret s + "!" }` or `let greet = fn(name) => ...`; IIFE: `((n) => ret ...)(name)`.
- Control flow: `if c { } els { }` — **`els`, not `else`**. `while c { }`, `for i -> 0..N { }` (range loops), `break`/`continue`.
- `match` on any value: `match v { 1 => ..., 4..9 => ..., "hi", "bye" => ..., _ => ... }` — ranges, comma-separated multi-patterns, `_` wildcard, no fallthrough error (silently skips unmatched).
- Structs: `struct Person { let name; fn new(n) => { ret Person { name: n } } }` — constructor via `Person::new(...)`, fields via `Person { name: n }` literal, methods access `self`.
- Operators: `+ - * / % **`, bitwise `<< >> & |`, compound `+= -= *= /= ++`, `== != < > <= >=`.
- Strings: interpolation `{var}` **only** through `io::print`/`io::println`/`io::format` (native `write` does not interpolate — use `+` concatenation). Methods: `len` (bytes), `substr(start, n)`, `split`, `join`, `ord`, `chr` (native), `to_upper`, `to_lower`, `trim`, `to_number` (null on failure), `replace`, `push`, `pop`, `includes`. Indexing `s[i]`, assignment `m[0] = "H"`, repeat `"=" * 8`.
- Lists: `[1, 2, 3]`, repeat `[0] * N`, indexing `board[i]`, same methods (`len`, `join`). Numbers auto-convert to strings in `+`.
- Comments: `//`.
- Imports: `import std::io` (module), `import std::io::print` (single item), `import "lib.iok" @ lib` (file, chainable: `lib::foo::add(...)`, relative to the importing file).
- Std APIs: `std::io` → `print`/`println`/`input`/`format`; `std::net` → `bind`, `connect`, `create_socket`, `http_get`; `std::fs` → `open`, `create`, `list_dir`, `exists`, `delete`, `append`; `std::ffi` → `load`, `sym`, `def_struct`, `struct_val`, `field`, `set_field`, `byref`, `nullptr`; File/Socket methods wrap the `__`-natives (e.g. `client.read()`, `file.write(data)`). Global natives: `write`, `readline`, `exit`, `chr`, `eval`.

## Conventions

- Keep the stdlib faithful: users interact with `std/` APIs, not the `__`-prefixed natives.
- String interning: new identifiers must go through `interner::intern`.
- Deps are minimal (`rustc-hash`, `lazy_static`, `libloading`, `libffi`) — don't add crates for things std can do.