local M = {}

function M.setup(opts)
  opts = opts or {}
  local binary = opts.cmd or "iok-lsp"

  vim.api.nvim_create_autocmd("FileType", {
    pattern = "iok",
    callback = function()
      if vim.fn.executable(binary) == 1 then
        vim.lsp.start({
          name = "iok-lsp",
          cmd = { binary },
          root_dir = vim.fs.dirname(vim.fs.find({ "Cargo.toml", ".git" }, { upward = true })[1]) or vim.fn.getcwd(),
          settings = {},
        })
      else
        vim.notify("[iok.nvim] iok-lsp binary not found in PATH", vim.log.levels.WARN)
      end
    end,
  })
end

return M
