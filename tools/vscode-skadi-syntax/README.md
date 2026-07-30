# Skadi VS Code Syntax

This folder contains the VS Code language support extension for the current
Skadi surface.

## What is included

- `package.json` (language registration)
- `language-configuration.json` (comments/brackets/autoclose)
- `syntaxes/skadi.tmLanguage.json` (TextMate grammar)
- `snippets/skadi.json` (workflow-oriented language snippets)

## Local usage

1. Open this folder in VS Code extension dev host mode (`F5`) from `tools/vscode-skadi-syntax`.
2. Open any `.skd` file.
3. Choose language mode `Skadi` if needed.

## Install from VSIX

1. Build package:
   - `npx @vscode/vsce package --allow-missing-repository`
2. Install in VS Code:
   - `code --install-extension skadi-syntax-0.3.7.vsix`

## File extensions and naming

- primary extension: `.skd`
- legacy extension: `.scadi`
- primary language mode: `Skadi`
- legacy alias kept only for backward compatibility: `Scadi`

## Current scope

- comments (`//`, `/* */`)
- strings with `__var__` interpolation token highlighting
- numbers
- declarations (`fn`, `struct`, `label`, `tag`) and function signatures
- control-flow and modifiers (`danger`, `new`, `on error`, `allow grow/drop`,
  `direct`, `view`, `move`, etc.)
- core type names, constants, and canonical aliases (`Bool`, `Char`, `PI`, `TAU`, `EPSILON`)
- builtins (`output`, `read`, `contains`, `len`, `slice`, `find`, `concat`, `args`, `fs.*`, math core)
- member calls/access (`.push()`, `.pop()`, `.field`, and custom methods like `.inc()`)
- struct literal field names, `ByteSize`, memory size literals and
  `memory.child`/`memory.static`
- `Time`, `Duration`, duration literals (`5ms`, `2s`, `3min`) and time builtins
- `Angle`, angle literals (`45deg`, `0.25rad`) and trigonometry integration
- `Vec2`, `Vec3`, `Vec4`, component fields and vector math builtins
- `Interrupt`, `interrupts.periodic`, and paired `on interrupt`
- `Color`, `Rect`, `Canvas`, `Window`, `windows.open`, and current drawing
  methods
- label members, struct field declarations, and typed variable declarations
- paired control forms like `on error`, `place in`, and `iterate ... as ...`
- operators and function call highlighting
- snippets for functions, error handling, regions, tasks/channels, interrupts,
  ownership borrows, and Canvas windows

## Next step

- Add diagnostics, navigation, and completion through an LSP integration.
