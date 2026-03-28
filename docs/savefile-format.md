# Victoria 2 Savefile Format Notes

## Basic structure

The savefile is made of statements.

Each statement has this shape:

`identifier = value`

## Values

A value can be one of:

- an identifier
- a string
- a numeric literal
- a block

## Tokens

### Identifier

An identifier is a series of characters that may include:

- letters
- digits
- `_`
- `-`
- `.`
- `:`

### String

A string is surrounded by double quotes.

Assumption for now:

- strings do not use escape sequences

Example:

`name = "United Kingdom"`

### Block

A block is surrounded by braces and contains statements.

Example:

```text
country = {
  tag = ENG
  name = "United Kingdom"
}
```

## Recursive structure

Blocks contain statements, and each statement is again:

`identifier = identifier / string / numeric literal / block`

This means the format is recursive and can be parsed as a tree of statements.

## Additional parsing assumptions

- Savefiles are large, so parser design should assume streaming or otherwise efficient parsing
- The parser can be lossy
- We should preserve statement order
- We assume keys do not repeat inside the same block
- If a key repeats inside the same block, the parser should panic for now
- There are no comments
