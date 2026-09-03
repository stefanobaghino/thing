# Editor support for ting

`ting.tmLanguage.json` is a TextMate grammar: comments, strings with
escape validation, numbers, keywords, the builtins, function
definitions and calls. Any editor that speaks TextMate grammars can
use it — no marketplace or extension install required.

## VS Code

A grammar becomes a tiny local extension: create
`~/.vscode/extensions/ting-lang/` containing this `package.json`
next to a copy of the grammar, then reload VS Code:

```json
{
  "name": "ting-lang",
  "version": "0.1.0",
  "engines": { "vscode": "^1.0.0" },
  "contributes": {
    "languages": [
      { "id": "ting", "extensions": [".ting"], "configuration": "./language-configuration.json" }
    ],
    "grammars": [
      { "language": "ting", "scopeName": "source.ting", "path": "./ting.tmLanguage.json" }
    ]
  }
}
```

Optional `language-configuration.json` for comments/brackets:

```json
{
  "comments": { "lineComment": "#" },
  "brackets": [["{", "}"], ["[", "]"], ["(", ")"]],
  "autoClosingPairs": [
    { "open": "{", "close": "}" },
    { "open": "[", "close": "]" },
    { "open": "(", "close": ")" },
    { "open": "\"", "close": "\"" }
  ]
}
```

## Sublime Text

`Preferences → Browse Packages…`, then copy `ting.tmLanguage.json`
into the `User/` directory. Sublime picks up `.tmLanguage.json`
grammars directly.

## Zed

Zed consumes TextMate grammars through its extension format; point a
local extension's `grammars` entry at this file. See Zed's docs on
"languages" for the two-file scaffold.

## Language server (LSP)

The `ting` binary doubles as a language server: `ting --lsp` speaks
JSON-RPC over stdio. On every open and change it publishes
diagnostics — lex, parse and compile errors; an error on an `import`
of a local file that has one; and the checker's warnings (a stdlib
module indexed with a name it does not export, an unused binding at
the top level or inside a block, an unused parameter, a name that
shadows a builtin). Beyond diagnostics it provides hover
(builtins, stdlib functions and the file's own), completion,
signature help, formatting, document and workspace symbols,
go-to-definition, references, highlights of the symbol under the
cursor, rename across open files (with a prepare step that declines
keywords and builtins), folding, document links on imports, and a
quickfix for misspelt stdlib members — thirteen capabilities, all
from the one binary with no configuration. Point any LSP client at it:

**Neovim** (built-in LSP):

```lua
vim.api.nvim_create_autocmd("FileType", {
  pattern = "ting",
  callback = function()
    vim.lsp.start({ name = "ting", cmd = { "ting", "--lsp" } })
  end,
})
vim.filetype.add({ extension = { ting = "ting" } })
```

**VS Code**: any generic LSP-client extension works — configure the
language id `ting` with server command `ting --lsp`.

**Zed**: add a language entry for `.ting` whose `language_servers`
command runs `ting --lsp`.
