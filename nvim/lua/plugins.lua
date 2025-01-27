return {
  -- Completion engine
  'hrsh7th/nvim-cmp',
  dependencies = {
    -- Completion sources
    'hrsh7th/cmp-buffer',       -- Words from current buffer
    'hrsh7th/cmp-path',         -- File paths
    'hrsh7th/cmp-nvim-lsp',     -- LSP-based completion
    'hrsh7th/cmp-nvim-lua',     -- Neovim Lua API completions

    -- Snippets
    'L3MON4D3/LuaSnip',         -- Snippet engine
    'saadparwaiz1/cmp_luasnip', -- Snippet completions
  },
}
