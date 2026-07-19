# Skadi VS Code Syntax (local draft)

This folder contains a local VS Code syntax-highlighting extension for Skadi.

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
   - `code --install-extension skadi-syntax-0.3.3.vsix`

## File extensions and naming

- primary extension: `.skd`
- legacy extension: `.scadi`
- primary language mode: `Skadi`
- legacy alias kept only for backward compatibility: `Scadi`

## Current scope

- comments (`//`, `/* */`)
- strings with `__var__` interpolation token highlighting
- numbers
- declarations (`fn`, `struct`, `label`) and better function signature coverage
- control-flow and modifiers (`danger`, `new`, `on error`, `allow`, `drop`, `direct`, etc.)
- core type names, constants, and canonical aliases (`Bool`, `Char`, `PI`, `TAU`, `EPSILON`)
- builtins (`output`, `read`, `contains`, `len`, `slice`, `find`, `concat`, `args`, `fs.*`, math core)
- member calls/access (`.push()`, `.pop()`, `.field`, and custom methods like `.inc()`)
- struct literal field names and memory size units (`16kb`, `8mb`, ...)
- `Time`, `Duration`, duration literals (`5ms`, `2s`, `3min`) and time builtins
- label members, struct field declarations, and typed variable declarations
- paired control forms like `on error`, `place in`, and `iterate ... as ...`
- operators and function call highlighting

## Next step

- Add snippets and diagnostics bridge once LSP is introduced.
