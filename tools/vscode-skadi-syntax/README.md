# Skadi VS Code Syntax (local draft)

This folder contains a minimal VS Code syntax-highlighting extension for Skadi.

## What is included

- `package.json` (language registration)
- `language-configuration.json` (comments/brackets/autoclose)
- `syntaxes/skadi.tmLanguage.json` (TextMate grammar)

## Local usage

1. Open this folder in VS Code extension dev host mode (`F5`) from `tools/vscode-skadi-syntax`.
2. Open any `.skd` file.
3. Choose language mode `Skadi` if needed.

## Install from VSIX

1. Build package:
   - `npx @vscode/vsce package --allow-missing-repository`
2. Install in VS Code:
   - `code --install-extension skadi-syntax-0.1.0.vsix`

## Current scope

- comments (`//`, `/* */`)
- strings with `__var__` interpolation token highlighting
- numbers
- declarations (`fn`, `struct`, `label`)
- control-flow and modifiers (`danger`, `new`, `on error`, etc.)
- core type names and constants
- builtins (`output`, `read`, `len`, `slice`, `find`, `concat`, `args`, `fs.*`)
- member calls/access (`.push()`, `.pop()`, `.field`, and custom methods like `.inc()`)
- operators and function call highlighting

## Next step

- Add better context-sensitive highlighting for declarations (`fn`, `struct`, `label`, `new`).
- Add snippets and diagnostics bridge once LSP is introduced.


