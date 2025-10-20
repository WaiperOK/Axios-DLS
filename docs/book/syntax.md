# Lexical and Syntactic Grammar

Axios follows a compact, line-oriented syntax that can be parsed without lookahead beyond a single line. This chapter codifies the lexical conventions and the context-free grammar implemented by the reference parser (`core/src/scenario.rs`).

## Lexical Conventions

- **Character Set**: All source files are UTF-8 encoded. Identifiers and directive keywords are restricted to ASCII letters, digits, hyphen (`-`), and underscore (`_`).
- **Whitespace**: Spaces, tabs, and blank lines separate tokens. Consecutive whitespace is collapsed and carries no semantic weight.
- **Comments**: Lines beginning with `#` or `//` are ignored. Inline comments are not currently supported; comments must occupy entire lines.
- **String Literals**: Unquoted strings extend to the end of the line or the next semicolon in property lists. Double-quoted strings (`"..."`) preserve embedded whitespace. Escape sequences are not yet interpreted; the content between quotes is used verbatim.
- **Shebang**: An optional leading `#!` line (for example `#!/usr/bin/env axion`) is ignored by the parser.

## High-Level Grammar

The grammar is expressed in Backus–Naur form with the following conventions:

- `identifier` matches `[A-Za-z0-9_-]+`.
- `string` denotes either a quoted string or an unquoted token as described above.
- `newline` represents the newline separator after each directive header.

```
scenario     ::= statement*
statement    ::= import_stmt
               | variable_stmt
               | asset_group_stmt
               | scan_stmt
               | script_stmt
               | report_stmt

import_stmt  ::= "import" string newline?

variable_stmt ::= "let" identifier "=" string newline?

asset_group_stmt ::= ("asset_group" | "group") identifier "{" property_list "}"
property_list    ::= (property (";" property)*)?
property         ::= identifier string

scan_stmt    ::= "scan" identifier scan_tool "{" scan_entry* "}"
scan_tool    ::= identifier
               | "using" identifier        ; convenience keyword
scan_entry   ::= identifier string
               | "}" "->" identifier       ; optional artifact label on closing line

script_stmt  ::= "script" identifier "{" script_entry* "}"
script_entry ::= identifier string
               | "}" "->" identifier

report_stmt  ::= "report" identifier "{" report_entry* "}"
report_entry ::= "include" identifier
```

## Parameter Canonicalisation

- Properties inside `asset_group`, `scan`, and `script` blocks are inserted into ordered maps with the textual key as provided. Keys must be unique within their block.
- Nested keys are expressed using dotted notation (`params.target`, `params.flags`). The parser does not treat dots specially; the runtime interprets specific keys by convention.
- Trailing semicolons inside `asset_group` blocks are optional. For readability, authors may place one property per line without semicolons or compress several properties into a single line separated by semicolons.

## Quoting Rules

- Double quotes must appear in balanced pairs. Partial quoting (e.g. `"open` or `open"`) is rejected.
- Quoted values cannot span multiple lines. Authors should use variables to compose large blocks of text if required.
- Unquoted values trim trailing whitespace but preserve internal spaces; for example `params.flags = -sV -O` associates the value `-sV -O` with the key.

## Identifier Resolution

- The parser does not enforce uniqueness of names across directives; conflicts are resolved dynamically during execution. Style guides should ensure meaningful, non-colliding names.
- Report `include` statements accept identifiers or literal artifact names. Because artifacts may be renamed through the `-> alias` syntax, report authors must reference the final alias rather than the original step name.

This grammar is intentionally conservative. Extensions such as multi-line strings, block comments, or nested modules will appear in future revisions once the implications for tooling and backwards compatibility are fully analysed.
