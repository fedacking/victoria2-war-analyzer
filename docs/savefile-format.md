# Victoria 2 Savefile Format Notes

## Basic structure

The savefile is made of statements.

Most statements have this shape:

`identifier = value`

Some rare statements can also be a bare identifier with no `= value`.

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

A block is surrounded by braces.

Blocks can contain either:

- statements
- bare values

Some blocks are list-like and contain only values, such as numeric histories.

Example:

```text
country = {
  tag = ENG
  name = "United Kingdom"
}
```

## Recursive structure

Statement blocks contain statements, and each statement is again:

`identifier = identifier / string / numeric literal / block`

Value-list blocks contain bare values directly inside the block.

This means the format is recursive and can be parsed as a tree.

## Additional parsing assumptions

- Savefiles are large, so parser design should assume streaming or otherwise efficient parsing
- The parser can be lossy
- We should preserve statement order
- Keys can repeat inside the same block
- The parser should preserve repeated keys in order
- Saves may end with one or more trailing `}` tokens after the real root content
- There are no comments
