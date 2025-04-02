# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build/Test Commands
- Build: `cargo build`
- Release build: `cargo build --release`
- Install from source: `cargo install --path .`
- Run: `cargo run -- [URL] [OPTIONS]`
- Lint: `cargo clippy -- -D warnings`
- Format: `cargo fmt -- --check`
- Test: `cargo test` (Note: Add tests in the future)
- Test single file: `cargo test -- path::to::test`

## Code Style Guidelines
- Error handling: Use `AppError` enum with `thiserror` for all errors
- Return types: Functions should return `Result<T, AppError>` where appropriate
- Imports: Group by std, external crates, then internal modules
- Security: Follow strict input validation and path sanitization
- Formatting: Follow rustfmt conventions with 4-space indentation
- Naming: Use snake_case for variables/functions, CamelCase for types
- Documentation: Add doc comments to public functions and types
- Feature flags: Use `#[cfg(feature = "pro")]` for premium features

Security is a top priority. Validate all inputs, prevent path traversal attacks, and use proper error handling.