# Contributing

## Getting Started

1. Clone the monorepo
2. Install Rust, Node.js, and Tauri prerequisites
3. Use `make all` to build everything, or see below for individual builds

## Adding Features

- **REST API**: Add endpoints to `keepkey-rest`, then update consumers (kkcli, vault)
- **UI**: Add features to `vault-ui`. Test in both kkcli and vault.
- **Firmware**: Develop in `firmware/` and test with both clients

## Testing

- Run `cargo test` in each Rust crate
- Run `npm test` or similar in `vault-ui`

## Pull Requests

- Ensure all builds pass (`make all`)
- Document changes in the appropriate doc
