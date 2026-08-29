# PrivID Telegram Bot (Rust)

A privacy-respecting identity verification Telegram bot built with Rust and teloxide. This bot integrates with Holonym to provide zero-knowledge proof-based identity verification without collecting or storing personal data.

## 🚀 Features

-   **Privacy-First Design**: Zero-knowledge proofs ensure no personal data is collected or stored
-   **Holonym Integration**: Seamless integration with Holonym's verification API
-   **Session Management**: In-memory session tracking for verification states
-   **Demo Mode**: Runs without API keys for testing and development
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
-   Optional: Holonym API Key for full verification features

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

# Optional (for full verification features)
HOLONYM_API_KEY=your_holonym_api_key_here

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
3. Use `/verify` to test verification (demo mode if no API key)
4. Use `/status` to check verification status
5. Use `/help` for command information

## 📚 Commands

| Command   | Description                              |
| --------- | ---------------------------------------- |
| `/start`  | Start the bot and see welcome message    |
| `/verify` | Begin identity verification with Holonym |
| `/status` | Check your verification status           |
| `/help`   | Show help information                    |

## 🔧 Configuration

### Environment Variables

-   `TELEGRAM_BOT_TOKEN` (required): Your Telegram bot token from BotFather
-   `HOLONYM_API_KEY` (optional): Holonym API key for full verification features
-   `RUST_LOG` (optional): Logging level (debug, info, warn, error)

### Running Modes

#### Demo Mode (Default)

-   Runs without Holonym API key
-   Shows verification flow without actual verification
-   Perfect for testing and development

#### Full Mode

-   Requires valid Holonym API key
-   Performs actual identity verification
-   Stores verification results in session state

## 🏗️ Architecture

### Project Structure

```
src/
├── main.rs          # Main bot logic and command handlers
├── holonym.rs       # Holonym API integration
└── state.rs         # Session and state management
```

### Key Components

#### Bot State Management (`state.rs`)

-   `UserSession`: Tracks user verification state and metadata
-   `VerificationState`: Enum for different verification states
-   `BotState`: Thread-safe session storage using RwLock

#### Holonym Integration (`holonym.rs`)

-   `HolonymClient`: HTTP client for Holonym API
-   Verification request/response structures
-   Mock verification for demo mode

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
    export HOLONYM_API_KEY=your_key  # optional
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
