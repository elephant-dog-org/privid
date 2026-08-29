# PrivID Telegram Bot (Rust)

A privacy-respecting identity verification Telegram bot built with Rust and teloxide. It reads Human ID (Holonym) zero-knowledge credentials straight from the Hub contract on Optimism, so it can confirm someone is a verified human without collecting or storing personal data.

## 🚀 Features

-   **Privacy-First Design**: Zero-knowledge proofs ensure no personal data is collected or stored
-   **On-chain Verification**: Reads Human ID SBTs (KYC, phone, ePassport, clean hands, biometrics) from Holonym Hub V3 on Optimism — no API keys
-   **ENS Identity Linking**: `/register name.eth` proves ownership through the `org.telegram` text record; no wallet signature
-   **Trust Checkpoint**: `/challenge` mints a single-use link; the counterparty taps it and you get the verdict by DM
-   **Mock Mode**: Runs without any RPC calls for testing and development
-   **Rich Commands**: Comprehensive command set with helpful responses
-   **Error Handling**: Robust error handling and user-friendly error messages
-   **Logging**: Comprehensive logging for debugging and monitoring

## 🛠️ Tech Stack

-   **Language**: Rust 2021 Edition
-   **Telegram Bot Framework**: teloxide 0.17.0
-   **Async Runtime**: tokio 1.0
-   **HTTP Client**: reqwest 0.12 (with rustls-tls)
-   **Serialization**: serde + serde_json
-   **Error Handling**: anyhow + thiserror
-   **Logging**: log + env_logger
-   **Configuration**: dotenv

## 📋 Prerequisites

-   Rust 1.70+ (install via [rustup](https://rustup.rs/))
-   Telegram Bot Token (get from [@BotFather](https://t.me/BotFather))
-   Nothing else — verification uses public Optimism and Ethereum RPC endpoints (override with `OPTIMISM_RPC_URL` / `ETHEREUM_RPC_URL`)

## 🚀 Quick Start

### 1. Clone and Setup

```bash
cd apps/telegram-bot
cp env.example .env
```

### 2. Configure Environment

Edit `.env` file with your credentials:

```bash
# Required
TELEGRAM_BOT_TOKEN=your_telegram_bot_token_here

# "mock" (no RPC calls) or "blockchain" (real Human ID SBT reads on Optimism)
VERIFICATION_MODE=blockchain

# Logging
RUST_LOG=info
```

### 3. Build and Run

```bash
# Development (recommended)
cargo run

# Production build
cargo build --release
./target/release/privid-telegram-bot

# Using Makefile
make run
```

### 3b. Run against the real chain

`VERIFICATION_MODE=blockchain` reads Human ID (Holonym) SBTs directly from the
Hub V3 contract on Optimism and resolves ENS on mainnet — public RPCs, no API
keys. Before a live session:

```bash
cargo test                                      # unit tests, offline
cargo test --test live_rpc_test -- --ignored    # hits the real RPCs
```

See `SMOKE_TEST.md` for the end-to-end script with real Telegram accounts.

### 4. Test the Bot

1. Find your bot on Telegram (using the username you set up with BotFather)
2. Send `/start` to begin
3. Use `/verify 0x...` to check a wallet's SBTs (mock data if `VERIFICATION_MODE=mock`)
4. Use `/status` to check verification status
5. Use `/help` for command information

## 📚 Commands

| Command                 | Description                                                        |
| ----------------------- | ------------------------------------------------------------------ |
| `/start`                | Welcome message (also handles `chg_` challenge links)              |
| `/challenge [note]`     | Mint a single-use verification link to send someone (DM only)      |
| `/challenges`           | List the challenges you've sent and their results                  |
| `/verifyme`             | See exactly what a challenger learns about you                     |
| `/verify <wallet>`      | Check any wallet's Human ID SBTs                                   |
| `/register <name.eth>`  | Link your ENS name to your Telegram account (DM only)              |
| `/deregister`           | Remove the link                                                    |
| `/whois @user`          | Look up someone's verification (or reply to their message)         |
| `/status`               | Your registration status                                           |
| `/help`                 | Help and safety rules                                              |

## 🔧 Configuration

### Environment Variables

-   `TELEGRAM_BOT_TOKEN` (required): Your Telegram bot token from BotFather
-   `VERIFICATION_MODE` (optional): `mock` (default) or `blockchain`
-   `OPTIMISM_RPC_URL`, `ETHEREUM_RPC_URL` (optional): override the public RPC endpoints
-   `HUB_CONTRACT_ADDRESS` (optional): Holonym Hub V3, defaults to `0x2AA822e264F8cc31A2b9C22f39e5551241e94DfB`
-   `API_BIND`, `API_PORT` (optional): HTTP lookup API, default `127.0.0.1:3141`
-   `RUST_LOG` (optional): Logging level (debug, info, warn, error)

### Running Modes

#### Mock Mode (Default)

-   No RPC calls; deterministic fake SBT results
-   Perfect for testing and development

#### Blockchain Mode

-   `VERIFICATION_MODE=blockchain`
-   Calls `getSBT(address, circuitId)` on Holonym Hub V3 on Optimism for each credential type, honouring expiry and revocation
-   Resolves ENS names and text records on Ethereum mainnet
-   Verify the endpoints are reachable with `cargo test --test live_rpc_test -- --ignored`

## 🏗️ Architecture

### Project Structure

```
src/
├── main.rs            # Command handlers, challenge flow, verdict formatting
├── config.rs          # Environment configuration
├── db.rs              # SQLite: identities, platform links, challenges
├── registry.rs        # Registry facade over the database
├── api.rs             # HTTP lookup API used by the browser extension
├── state.rs           # Session state
├── blockchain/        # Hub V3 ABI encoding, JSON-RPC client, SBT types
├── ens/               # ENS namehash + resolver (address + text records)
└── verification/      # VerificationProvider trait, mock + blockchain impls
```

### Key Components

#### Bot State Management (`state.rs`)

-   `UserSession`: Tracks user verification state and metadata
-   `VerificationState`: Enum for different verification states
-   `BotState`: Thread-safe session storage using RwLock

#### Verification Providers (`verification/`, `blockchain/`)

-   `VerificationProvider` trait — the seam for adding credential sources
-   `BlockchainVerificationProvider`: reads SBTs from Hub V3 via `eth_call`
-   `MockVerificationProvider`: deterministic results for development

#### Main Bot Logic (`main.rs`)

-   Command definitions and handlers
-   User session management
-   Rich message formatting
-   Error handling and logging

## 🔒 Privacy Features

-   **Zero-Knowledge Proofs**: Verification without revealing personal data
-   **No Data Storage**: Sessions are in-memory only (no persistence)
-   **Anonymous Verification**: No personal information required
-   **End-to-End Privacy**: Complete privacy throughout the verification process

## 🧪 Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Check for issues
cargo check
cargo clippy
```

### Testing

```bash
# Run tests
cargo test

# Run with specific log level
RUST_LOG=debug cargo run
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Check for security issues
cargo audit
```

### Using Makefile

```bash
# Setup environment
make setup

# Run in development
make run

# Build release version
make release

# Run tests
make test

# Code quality checks
make check
```

## 🚀 Deployment

### Local Development

```bash
# Simple development run
cargo run

# Using Makefile
make run
```

### Production Deployment

1. **Build for production**:

    ```bash
    cargo build --release
    # or
    make release
    ```

2. **Deploy binary** to your server

3. **Set environment variables**:

    ```bash
    export TELEGRAM_BOT_TOKEN=your_token
    export VERIFICATION_MODE=blockchain
    export RUST_LOG=info
    ```

4. **Run the bot**:
    ```bash
    ./target/release/privid-telegram-bot
    ```

### Systemd Service (Linux)

Create `/etc/systemd/system/privid-bot.service`:

```ini
[Unit]
Description=PrivID Telegram Bot
After=network.target

[Service]
Type=simple
User=privid
WorkingDirectory=/opt/privid-bot
Environment=TELEGRAM_BOT_TOKEN=your_token_here
Environment=RUST_LOG=info
ExecStart=/opt/privid-bot/privid-telegram-bot
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start the service:

```bash
sudo systemctl enable privid-bot
sudo systemctl start privid-bot
```

## 🔮 Future Enhancements

-   [ ] Database persistence for sessions
-   [ ] Redis integration for distributed deployment
-   [ ] Webhook support for production scaling
-   [ ] Admin commands for bot management
-   [ ] Analytics and monitoring
-   [ ] Multi-language support
-   [ ] Integration with other verification providers
-   [ ] Docker support (simplified, working version)

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Run `cargo test` and `cargo clippy`
6. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](../../LICENSE) file for details.

## 🆘 Support

-   Create an issue for bugs or feature requests
-   Check the [PrivID documentation](../../docs/) for more information
-   Join our community for discussions and updates

## 🔗 Related Projects

-   [PrivID Frontend](../../apps/frontend/) - Web interface for PrivID
-   [PrivID Extension](../../apps/extension/) - Browser extension for PrivID
-   [PrivID Documentation](../../docs/) - Comprehensive documentation
