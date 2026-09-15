# Contributing to Web3 Suite Identity Contracts

Thank you for your interest in contributing! This document provides guidelines and instructions for contributing to this project.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/web3-suite-identity-contracts.git`
3. Create a feature branch: `git checkout -b feature/amazing-feature`
4. Make your changes
5. Run tests: `cargo test`
6. Commit your changes: `git commit -m 'Add amazing feature'`
7. Push to the branch: `git push origin feature/amazing-feature`
8. Open a Pull Request

## Development Setup

### Prerequisites

- Rust (latest stable)
- Soroban CLI (`cargo install --locked soroban-cli`)
- Stellar CLI (`cargo install --locked stellar-cli`)

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Contract Compilation

```bash
soroban contract build
```

## Code Style

- Follow standard Rust conventions
- Use `cargo fmt` to format code
- Use `cargo clippy` to lint code
- Write meaningful commit messages
- Include tests for new functionality

## Pull Request Process

1. Update documentation if needed
2. Add tests for new features
3. Ensure all tests pass
4. Request review from maintainers
5. Address feedback promptly

## Contract Development Guidelines

- Always validate inputs at system boundaries
- Use descriptive error types with `#[contracterror]`
- Emit events for all state-changing operations
- Write comprehensive tests including edge cases
- Document public functions with doc comments

## Reporting Issues

- Use GitHub Issues for bug reports
- Include reproduction steps
- Provide environment details (Rust version, Soroban version)
- Attach relevant logs or error messages

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
