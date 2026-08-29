-- Neovim filetype plugin for iok

-- Set up comments
vim.bo.commentstring = "// %s"

-- Standard indentation (2 spaces is common for iok examples)
vim.bo.expandtab = true
vim.bo.shiftwidth = 2
vim.bo.tabstop = 2
vim.bo.softtabstop = 2

-- LSP is started via require("iok").setup() in lua/iok/init.lua
-- (removed auto-start here to avoid duplicate clients + lsp.log spam)
