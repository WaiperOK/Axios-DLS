# Appendix A: Grammar Reference

This appendix restates the Axios grammar in an implementation-neutral format suitable for parser generation. It refines the presentation from [Lexical and Syntactic Grammar](syntax.md) and makes explicit the token definitions.

## Tokens

- `IDENT` — `[A-Za-z_][A-Za-z0-9_-]*`
- `STRING` — either a quoted string (`"[^"\n]*"`) without embedded newlines or an unquoted sequence matching `[^#/\s][^\n]*` trimmed of trailing whitespace.
- `NEWLINE` — `\r?\n`
- `COMMENT` — lines beginning with `#` or `//` that extend to `NEWLINE`.
- `ARROW` — `->`
- `LBRACE` — `{`
- `RBRACE` — `}`
- `EQUALS` — `=`

## Grammar

```
scenario      ::= (statement | comment | blank)* EOF
statement     ::= import_stmt
                | variable_stmt
                | asset_group_stmt
                | scan_stmt
                | script_stmt
                | report_stmt

import_stmt   ::= "import" STRING NEWLINE?

variable_stmt ::= "let" IDENT EQUALS STRING NEWLINE?

asset_group_stmt ::= ("asset_group" | "group") IDENT LBRACE asset_group_body RBRACE
asset_group_body ::= (asset_property (separator asset_property)*)?
asset_property   ::= IDENT STRING
separator        ::= ";" | NEWLINE

scan_stmt     ::= "scan" IDENT scan_tool_spec LBRACE scan_body RBRACE
scan_tool_spec ::= IDENT
                 | "using" IDENT
scan_body     ::= (scan_property NEWLINE?)* scan_close
scan_property ::= IDENT STRING
scan_close    ::= RBRACE (ARROW IDENT)?

script_stmt   ::= "script" IDENT LBRACE script_body RBRACE
script_body   ::= (script_property NEWLINE?)* script_close
script_property ::= IDENT STRING
script_close  ::= RBRACE (ARROW IDENT)?

report_stmt   ::= "report" IDENT LBRACE report_body RBRACE
report_body   ::= (report_include NEWLINE?)* RBRACE
report_include ::= "include" IDENT

comment       ::= COMMENT NEWLINE?
blank         ::= NEWLINE
```

This grammar is LL(1) and well-suited to deterministic recursive-descent parsers. Future language extensions should preserve these characteristics to maintain ease of implementation and readability.
