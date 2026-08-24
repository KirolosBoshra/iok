-- Neovim filetype plugin for iok

-- Set up comments
vim.bo.commentstring = "// %s"

-- Standard indentation (2 spaces is common for iok examples)
vim.bo.expandtab = true
vim.bo.shiftwidth = 2
vim.bo.tabstop = 2
vim.bo.softtabstop = 2

-- Try starting the built-in LSP client if the iok-lsp binary is available
local binary = "iok-lsp"
if vim.fn.executable(binary) == 1 then
  vim.lsp.start({
    name = "iok-lsp",
    cmd = { binary },
    root_dir = vim.fs.dirname(vim.fs.find({ "Cargo.toml", ".git" }, { upward = true })[1]) or vim.fn.getcwd(),
    settings = {},
  })
end
