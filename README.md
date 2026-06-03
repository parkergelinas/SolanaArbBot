# SolanaArbBot

A high-performance arbitrage detection and execution bot for the Solana blockchain. This project identifies profitable trading opportunities across Solana DEXs and executes atomic transactions to capture arbitrage spreads.

## Project Structure

This is a **Rust workspace** with modular components:

- **`accounts`** – Account data structures and utilities for Solana account handling
- **`common`** – Shared utilities, error types, and common functionality
- **`decoder`** – On-chain instruction and data decoding
- **`engine`** – Core arbitrage detection logic
- **`execution`** – Transaction building and submission
- **`graph`** – Price graph and relationship modeling between tokens/markets
- **`pricing`** – Real-time pricing and feed aggregation
- **`risk`** – Risk assessment and position management
- **`routing`** – Trade routing and path optimization
- **`rpc_client`** – Solana RPC client wrapper
- **`stream`** – Real-time blockchain data streaming

## Getting Started

### Prerequisites

- Rust 1.83+ (installed via `rustc` and `cargo`)
- Node.js v22 (optional, for auxiliary tooling)

### Building

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

## Configuration

Configuration details and runtime instructions will be documented as the project develops.

## License

MIT

## Contributing

See `AGENTS.md` for guidance on working with AI agents in this repository.
