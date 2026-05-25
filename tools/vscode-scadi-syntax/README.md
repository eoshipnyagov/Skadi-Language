# Scadi VS Code Syntax (local draft)

This folder contains a minimal VS Code syntax-highlighting extension for Scadi.

## What is included

- `package.json` (language registration)
- `language-configuration.json` (comments/brackets/autoclose)
- `syntaxes/scadi.tmLanguage.json` (TextMate grammar)

## Local usage

1. Open this folder in VS Code extension dev host mode (`F5`) from `tools/vscode-scadi-syntax`.
2. Open any `.scadi` file.
3. Choose language mode `Scadi` if needed.

## Install from VSIX

1. Build package:
   - `npx @vscode/vsce package --allow-missing-repository`
2. Install in VS Code:
   - `code --install-extension scadi-syntax-0.1.0.vsix`

## Current scope

- comments (`//`, `/* */`)
- strings with `__var__` interpolation token highlighting
- numbers
- declarations (`fn`, `struct`, `label`)
- control-flow and modifiers (`danger`, `new`, `on error`, etc.)
- core type names and constants
- builtins (`output`, `read`, `len`, `slice`, `find`, `concat`, `args`, `fs.*`)
- member calls/access (`.push()`, `.pop()`, `.signal()`, `.field`)
- operators and function call highlighting

## Next step

- Add better context-sensitive highlighting for declarations (`fn`, `struct`, `label`, `new`).
- Add snippets and diagnostics bridge once LSP is introduced.
