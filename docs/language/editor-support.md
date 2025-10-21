# Editor Support and Syntax Highlighting

Axion scenarios use the `.ax` (or `.axion`) extension. The repository ships a TextMate grammar that can be consumed by editors with TextMate-compatible highlighting, including Visual Studio Code, Sublime Text, and JetBrains IDEs.

## Visual Studio Code

1. Copy `tools/syntax/axion.tmLanguage.json` into your VS Code extensions directory, e.g.:
   ```
   mkdir -p ~/.vscode/extensions/axion.dsl/syntaxes
   cp tools/syntax/axion.tmLanguage.json ~/.vscode/extensions/axion.dsl/syntaxes/axion.tmLanguage.json
   ```
2. Create `package.json` in `~/.vscode/extensions/axion.dsl/` with:
   ```json
   {
     "name": "axion-dsl",
     "displayName": "Axion DSL",
     "version": "0.0.1",
     "publisher": "local",
     "engines": { "vscode": "^1.70.0" },
     "contributes": {
       "languages": [
         {
           "id": "axion",
           "aliases": ["Axion DSL", "axion"],
           "extensions": [".ax", ".axion"]
         }
       ],
       "grammars": [
         {
           "language": "axion",
           "scopeName": "source.axion",
           "path": "./syntaxes/axion.tmLanguage.json"
         }
       ]
     }
   }
   ```
3. Restart VS Code. Files ending with `.ax` or `.axion` should now highlight directives, variables, and strings.

## Sublime Text

1. Copy `tools/syntax/axion.tmLanguage.json` to `Packages/User/Axion.tmLanguage.json`.
2. Use `View → Syntax → Open all with current extension as... → Axion`.

## JetBrains IDEs

1. Install the **TextMate Bundles** plugin.
2. Open `Settings → Editor → TextMate Bundles` and add the directory containing `axion.tmLanguage.json`.
3. Associate the `.ax` extension with the new bundle.

## Customisation

The grammar highlights:

- Keywords `import`, `let`, `group`, `asset_group`, `scan`, `script`, `report`, `include`.
- Interpolated variables `${var}`.
- Single- and double-quoted strings.
- Numeric literals and arrow operators.

Adjust the repository patterns to expand or tweak highlighting for additional constructs.
