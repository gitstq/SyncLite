# Contributing to SyncLite

Thank you for your interest in contributing to SyncLite! We welcome contributions from the community.

## How to Contribute

1. **Fork the repository** and create your branch from `main`.
2. **Make your changes** and ensure they follow our coding standards.
3. **Write tests** for any new functionality.
4. **Update documentation** as needed.
5. **Submit a pull request** with a clear description of your changes.

## Development Setup

```bash
git clone https://github.com/gitstq/SyncLite.git
cd SyncLite
cargo build --all-features
```

## Code Style

- Follow Rust naming conventions
- Run `cargo fmt` before committing
- Run `cargo clippy` to check for linting issues
- Ensure all tests pass with `cargo test --all-features`

## Commit Message Format

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `style:` Code style changes (formatting)
- `refactor:` Code refactoring
- `test:` Test changes
- `chore:` Build process or auxiliary tool changes

## Reporting Issues

When reporting issues, please include:
- SyncLite version
- Operating system
- Steps to reproduce
- Expected vs actual behavior
- Any relevant logs or error messages

## Code of Conduct

Be respectful and constructive in all interactions.
