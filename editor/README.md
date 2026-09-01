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
