.PHONY: help build run test clean fmt clippy docker-build docker-run setup

# Default target
help:
	@echo "PrivID Telegram Bot - Available commands:"
	@echo "  setup      - Set up environment and dependencies"
	@echo "  build      - Build the project"
	@echo "  run        - Run the bot in development mode"
	@echo "  test       - Run tests"
	@echo "  clean      - Clean build artifacts"
	@echo "  fmt        - Format code"
	@echo "  clippy     - Run clippy linter"
	@echo "  docker-build - Build Docker image"
	@echo "  docker-run   - Run with Docker Compose"
	@echo "  release    - Build release version"

# Setup environment
setup:
	@echo "Setting up PrivID Telegram Bot..."
	@if [ ! -f .env ]; then \
		echo "Creating .env file from template..."; \
		cp env.example .env; \
		echo "Please edit .env file with your credentials"; \
	else \
		echo ".env file already exists"; \
	fi
	@echo "Setup complete!"

# Build the project
build:
	@echo "Building PrivID Telegram Bot..."
	cargo build

# Run in development mode
run:
	@echo "Running PrivID Telegram Bot in development mode..."
	cargo run

# Run tests
test:
	@echo "Running tests..."
	cargo test

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean

# Format code
fmt:
	@echo "Formatting code..."
	cargo fmt

# Run clippy linter
clippy:
	@echo "Running clippy..."
	cargo clippy

# Build release version
release:
	@echo "Building release version..."
	cargo build --release
	@echo "Release binary available at: target/release/privid-telegram-bot"

# Docker commands removed - use cargo run instead

# Check code quality
check: fmt clippy test
	@echo "Code quality check complete!"

# Full development workflow
dev: setup build test
	@echo "Development setup complete! Run 'make run' to start the bot." 