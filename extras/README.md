# IOk Editor Extras

This directory contains the Language Server Protocol (LSP) server and Neovim integration for the **iok** programming language.

---

## 1. LSP Server (`iok-lsp`)

The `iok-lsp` is a fast, robust Language Server built in Rust that provides rich editor support for the **iok** language, including:
- **Pre-runtime Static Type Inference**: Predicts the types of variables, parameters, expressions, struct instances, functions, and standard library members.
- **Autocompletion**:
  - Contextual variables with their inferred types.
  - Member access (`obj.`) completion for String, List, and Struct instance fields and methods.
  - Namespace (`mod::`) completion for built-in modules (`io`, `fs`, `net`, `ffi`) and custom imported modules.
  - Snippets for function calls and keywords.
- **Syntax Diagnostics**: Highlights unclosed brackets, parentheses, curly braces, and unmatched delimiters with editor squigglies.
- **Hover Information**: Displays variable types, detailed struct layouts, and function signatures.
- **Go-to-Definition**: Jump directly to variable declarations, parameters, struct definitions, or functions.
- **Document Symbols**: Navigable list of all declared variables, structs, and functions in the current file.

### Installation
From the root of the `iok` repository, compile and install the LSP server to your system's PATH:

```bash
cargo install --path extras/lsp
```

Ensure `iok-lsp` is executable and available in your terminal:
```bash
iok-lsp --version # or just run to check if it launches (it will wait on stdin)
```

---

## 2. Neovim Integration (`iok.nvim`)

The `neovim/` directory is a complete, lightweight Neovim plugin that provides syntax highlighting, automatic filetype detection (`*.iok`), indentation settings, and automatic LSP attachment.

### Directory Structure
- `ftdetect/iok.vim`: Detects `*.iok` files and sets `filetype=iok`.
- `syntax/iok.vim`: High-quality syntax highlighting for keywords, operators, comments, PascalCase type/struct names, string interpolation (`"Hi {name}"`), and standard modules.
- `ftplugin/iok.lua`: Configures buffer-local settings (e.g., `commentstring = "// %s"`) and triggers the built-in LSP client.
- `lua/iok/init.lua`: Setup module for customizable configurations.

---

## 3. Configuration in Neovim

### Option A: Manual Installation (using Lazy.nvim)

Add the following to your Neovim configurations (e.g. `init.lua` or your `plugins/` list):

```lua
-- Using lazy.nvim
{
  "iok",
  dir = "/path/to/iok/extras/neovim", -- Absolute path to the extras/neovim folder
  config = function()
    require("iok").setup({
      -- cmd = "iok-lsp" -- Option to specify different binary path
    })
  end,
  ft = "iok",
}
```

### Option B: Built-in Automatic LSP Start

Because `ftplugin/iok.lua` calls `vim.lsp.start` automatically when an `iok` file is opened, you don't even need to run `setup()`. Simply add the `extras/neovim` directory to your Neovim runtimepath (`rtp`):

```lua
vim.opt.rtp:append("/path/to/iok/extras/neovim")
```

Once loaded, opening any `.iok` file will instantly enable:
1. Syntax highlighting and correct comment shortcuts (`gcc`).
2. Type inference on hover (`K` / `vim.lsp.buf.hover()`).
3. Auto-completions (via Omni-completion `<C-x><C-o>` or any standard autocomplete engine like `nvim-cmp` / `coq` / `ddc`).
4. Go-to-Definition (`gd` / `vim.lsp.buf.definition()`).
5. Document Outline navigation.
