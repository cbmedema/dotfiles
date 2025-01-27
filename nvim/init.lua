-- ~/.config/nvim/init.lua
-- Set clipboard to use wl-clipboard (Wayland)
vim.g.clipboard = {
  name = 'wl-clipboard (Wayland)',
  copy = {
    ["+"] = 'wl-copy --foreground --type text/plain',
    ["*"] = 'wl-copy --foreground --primary --type text/plain',
  },
  paste = {
    ["+"] = 'wl-paste --no-newline',
    ["*"] = 'wl-paste --no-newline --primary',
  },
  cache_enabled = true
}

-- Use the system clipboard by default
vim.opt.clipboard:append('unnamedplus')  -- Use '+' register
vim.opt.clipboard:append('unnamed')      -- Use '*' register

-- removes x to improve dvorak neovim usage
vim.keymap.set('n', 'x', '<Nop>')  -- Disable normal mode x
vim.keymap.set('n', 'X', '<Nop>')  -- Disable normal mode X

local lazypath = vim.fn.stdpath("data") .. "/lazy/lazy.nvim"
if not vim.loop.fs_stat(lazypath) then
  vim.fn.system({
    "git",
    "clone",
    "--filter=blob:none",
    "https://github.com/folke/lazy.nvim.git",
    "--branch=stable",
    lazypath,
  })
end
vim.opt.rtp:prepend(lazypath)

require("lazy").setup({
  -- Gen.nvim configuration
  {
    "David-Kunz/gen.nvim",
    opts = {
      model = "deepseek-r1:14b",
      quit_map = "q",
      retry_map = "<c-r>",
      accept_map = "<c-cr>",
      host = "localhost",
      port = "11434",
      display_mode = "float",
      show_prompt = true,
      show_model = true,
      no_auto_close = false,
      file = false,
      hidden = false,
      init = function(options) pcall(io.popen, "ollama serve > /dev/null 2>&1 &") end,
      command = function(options)
        local body = {model = options.model, stream = true}
        return "curl --silent --no-buffer -X POST http://" .. options.host .. ":" .. options.port .. "/api/chat -d $body"
      end,
      result_filetype = "markdown",
      debug = false
    }
  },
  -- LSP and completion plugins
  {
    'williamboman/mason.nvim',
    config = function()
      require('mason').setup()
      require('mason-lspconfig').setup({
        ensure_installed = { 'lua_ls', 'pyright' }
      })
    end
  },
  'williamboman/mason-lspconfig.nvim',
  'neovim/nvim-lspconfig',
  'hrsh7th/nvim-cmp',
  'hrsh7th/cmp-nvim-lsp',
  'L3MON4D3/LuaSnip',
  'saadparwaiz1/cmp_luasnip',
})

-- LSP Setup (after plugin loading)
local lspconfig = require('lspconfig')
local capabilities = require('cmp_nvim_lsp').default_capabilities()

lspconfig.lua_ls.setup({ capabilities = capabilities })
lspconfig.pyright.setup({ capabilities = capabilities })

-- nvim-cmp configuration
local cmp = require('cmp')

cmp.setup({
  snippet = {
    expand = function(args)
      require('luasnip').lsp_expand(args.body)
    end,
  },
  mapping = cmp.mapping.preset.insert({
    ['<C-Space>'] = cmp.mapping.complete(),
    ['<CR>'] = cmp.mapping.confirm({ select = true }),
  }),
  sources = cmp.config.sources({
    { name = 'nvim_lsp' },
    { name = 'luasnip' },
  }, {
    { name = 'buffer' },
  })
})
