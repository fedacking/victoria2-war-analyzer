# Agents

## Git workflow

This is a single-user repository.

- Make changes directly on the default branch
- Do not create feature branches unless explicitly requested

## Rust workflow

Use Rust for as much of the application as possible.

- Keep business logic in Rust
- Keep parsing and data processing in Rust
- Keep complex functions in Rust
- Use React mainly for basic UI interactions and presentation

For every Rust code change:

- Run `cargo fmt`
- Run `cargo check`
