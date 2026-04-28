# AgentLink Chatbot Agent — Makefile
# =============================================================================

# Colors for terminal output
RED    := \033[0;31m
GREEN  := \033[0;32m
YELLOW := \033[0;33m
BLUE   := \033[0;34m
BOLD   := \033[1m
RESET  := \033[0m

# Project configuration
PROJECT_NAME := chatbot-agent
RUST_VERSION_MIN := 1.75
SDK_PATH := ../../agentlink-rust-sdk

# =============================================================================
# Default target
# =============================================================================
.PHONY: help
help: ## Show this help message
	@echo ""
	@echo "$(BOLD)AgentLink Chatbot Agent — Available Commands$(RESET)"
	@echo "======================================================================"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-15s$(RESET) %s\n", $$1, $$2}'
	@echo ""

# =============================================================================
# Pre-flight checks
# =============================================================================
.PHONY: check

check: ## Run all pre-flight checks before starting the agent
	@echo ""
	@echo "$(BOLD)🔍 Running pre-flight checks for $(PROJECT_NAME)...$(RESET)"
	@echo "======================================================================"

	@# --- Check 1: cargo installed ---
	@echo -n "  [1/6] Checking Cargo (Rust toolchain) ... "
	@if command -v cargo >/dev/null 2>&1; then \
		echo "$(GREEN)✓ OK$(RESET)"; \
	else \
		echo "$(RED)✗ FAILED$(RESET)"; \
		echo ""; \
		echo "$(RED)$(BOLD)Pre-flight check failed.$(RESET)"; \
		echo "  → Cargo is not installed or not in PATH."; \
		echo "  → Install Rust: https://rustup.rs/"; \
		echo ""; \
		exit 1; \
	fi

	@# --- Check 2: rustc version >= 1.75 ---
	@echo -n "  [2/6] Checking Rust version (>= $(RUST_VERSION_MIN)) ... "
	@if command -v rustc >/dev/null 2>&1; then \
		CURRENT_VERSION=$$(rustc --version | sed 's/rustc //'); \
		MIN_VERSION_NUM=$$(echo "$(RUST_VERSION_MIN)" | tr -d '.'); \
		CURRENT_VERSION_NUM=$$(echo "$$CURRENT_VERSION" | sed 's/\.//g' | cut -c1-3); \
		if [ "$$CURRENT_VERSION_NUM" -ge "$$MIN_VERSION_NUM" ] 2>/dev/null; then \
			echo "$(GREEN)✓ OK$(RESET) (found $$CURRENT_VERSION)"; \
		else \
			echo "$(RED)✗ FAILED$(RESET) (found $$CURRENT_VERSION)"; \
			echo ""; \
			echo "$(RED)$(BOLD)Pre-flight check failed.$(RESET)"; \
			echo "  → Rust version $$CURRENT_VERSION is too old."; \
			echo "  → Required: >= $(RUST_VERSION_MIN)"; \
			echo "  → Update Rust: rustup update"; \
			echo ""; \
			exit 1; \
		fi; \
	else \
		echo "$(RED)✗ FAILED$(RESET)"; \
		echo ""; \
		echo "$(RED)$(BOLD)Pre-flight check failed.$(RESET)"; \
		echo "  → rustc is not installed or not in PATH."; \
		echo ""; \
		exit 1; \
	fi

	@# --- Check 3: SDK dependency exists ---
	@echo -n "  [3/6] Checking agentlink-rust-sdk dependency ... "
	@if [ -d "$(SDK_PATH)" ] && [ -f "$(SDK_PATH)/Cargo.toml" ]; then \
		echo "$(GREEN)✓ OK$(RESET)"; \
	else \
		echo "$(RED)✗ FAILED$(RESET)"; \
		echo ""; \
		echo "$(RED)$(BOLD)Pre-flight check failed.$(RESET)"; \
		echo "  → agentlink-rust-sdk not found at $(SDK_PATH)."; \
		if [ -f "../../.gitmodules" ] && grep -q "agentlink-rust-sdk" ../../.gitmodules 2>/dev/null; then \
			echo "  → This project uses a git submodule. Run the following from the repository root:"; \
			echo "      git submodule update --init agentlink-rust-sdk"; \
		else \
			echo "  → Ensure the SDK repository is cloned alongside this project:"; \
			echo "      $(SDK_PATH)/"; \
		fi; \
		echo ""; \
		exit 1; \
	fi

	@# --- Check 4: .env file exists ---
	@echo -n "  [4/6] Checking .env file ... "
	@if [ -f ".env" ]; then \
		echo "$(GREEN)✓ OK$(RESET)"; \
	else \
		echo "$(RED)✗ FAILED$(RESET)"; \
		echo ""; \
		echo "$(RED)$(BOLD)Pre-flight check failed.$(RESET)"; \
		echo "  → .env file is missing."; \
		echo "  → Copy the example and configure your API keys:"; \
		echo "      cp .env.example .env"; \
		echo "  → Then edit .env and set your AGENTLINK_API_KEY and DEEPSEEK_API_KEY."; \
		echo ""; \
		exit 1; \
	fi

	@# --- Check 5: AGENTLINK_API_KEY configured ---
	@echo -n "  [5/6] Checking AGENTLINK_API_KEY ... "
	@AGENTLINK_KEY=$$(grep -E '^AGENTLINK_API_KEY=' .env 2>/dev/null | cut -d'=' -f2- | tr -d ' "'); \
	if [ -z "$$AGENTLINK_KEY" ]; then \
		echo "$(RED)✗ FAILED$(RESET)"; \
		echo ""; \
		echo "$(RED)$(BOLD)Pre-flight check failed.$(RESET)"; \
		echo "  → AGENTLINK_API_KEY is not set in .env."; \
		echo "  → Get your API key from the AgentLink platform and add it to .env:"; \
		echo "      AGENTLINK_API_KEY=sk_your_actual_key_here"; \
		echo ""; \
		exit 1; \
	elif echo "$$AGENTLINK_KEY" | grep -qiE 'your.*key|placeholder|example|xxxx'; then \
		echo "$(RED)✗ FAILED$(RESET)"; \
		echo ""; \
		echo "$(RED)$(BOLD)Pre-flight check failed.$(RESET)"; \
		echo "  → AGENTLINK_API_KEY appears to be a placeholder value: $$AGENTLINK_KEY"; \
		echo "  → Replace it with your actual API key from the AgentLink platform."; \
		echo ""; \
		exit 1; \
	else \
		echo "$(GREEN)✓ OK$(RESET)"; \
	fi

	@# --- Check 6: DEEPSEEK_API_KEY configured ---
	@echo -n "  [6/6] Checking DEEPSEEK_API_KEY ... "
	@DEEPSEEK_KEY=$$(grep -E '^DEEPSEEK_API_KEY=' .env 2>/dev/null | cut -d'=' -f2- | tr -d ' "'); \
	if [ -z "$$DEEPSEEK_KEY" ]; then \
		echo "$(RED)✗ FAILED$(RESET)"; \
		echo ""; \
		echo "$(RED)$(BOLD)Pre-flight check failed.$(RESET)"; \
		echo "  → DEEPSEEK_API_KEY is not set in .env."; \
		echo "  → Get your API key from https://platform.deepseek.com/ and add it to .env:"; \
		echo "      DEEPSEEK_API_KEY=sk-your-actual-deepseek-key"; \
		echo ""; \
		exit 1; \
	elif echo "$$DEEPSEEK_KEY" | grep -qiE 'your.*key|placeholder|example|xxxx'; then \
		echo "$(RED)✗ FAILED$(RESET)"; \
		echo ""; \
		echo "$(RED)$(BOLD)Pre-flight check failed.$(RESET)"; \
		echo "  → DEEPSEEK_API_KEY appears to be a placeholder value: $$DEEPSEEK_KEY"; \
		echo "  → Replace it with your actual DeepSeek API key."; \
		echo ""; \
		exit 1; \
	else \
		echo "$(GREEN)✓ OK$(RESET)"; \
	fi

	@echo ""
	@echo "$(GREEN)$(BOLD)✓ All pre-flight checks passed.$(RESET)"
	@echo "======================================================================"
	@echo ""

# =============================================================================
# Build targets
# =============================================================================
.PHONY: build

build: check ## Build the chatbot agent in debug mode
	@echo "$(BOLD)🔨 Building $(PROJECT_NAME)...$(RESET)"
	cargo build
	@echo ""
	@echo "$(GREEN)$(BOLD)✓ Build complete.$(RESET)"

.PHONY: build-release

build-release: check ## Build the chatbot agent in release mode
	@echo "$(BOLD)🔨 Building $(PROJECT_NAME) (release)...$(RESET)"
	cargo build --release
	@echo ""
	@echo "$(GREEN)$(BOLD)✓ Release build complete.$(RESET)"
	@echo "  Binary: $(BLUE)target/release/chatbot-agent$(RESET)"

# =============================================================================
# Run targets
# =============================================================================
.PHONY: start

start: check ## Run pre-flight checks and start the chatbot agent
	@echo "$(BOLD)🚀 Starting $(PROJECT_NAME)...$(RESET)"
	@echo "======================================================================"
	@echo ""
	cargo run

.PHONY: run

run: ## Start the chatbot agent without pre-flight checks (fast)
	cargo run

.PHONY: run-release

run-release: ## Run the release binary directly
	@if [ ! -f "target/release/chatbot-agent" ]; then \
		echo "$(RED)Release binary not found. Run 'make build-release' first.$(RESET)"; \
		exit 1; \
	fi
	./target/release/chatbot-agent

# =============================================================================
# Development helpers
# =============================================================================
.PHONY: clean

clean: ## Clean build artifacts
	cargo clean
	@echo "$(GREEN)✓ Build artifacts cleaned.$(RESET)"

.PHONY: fmt

fmt: ## Format Rust source code
	cargo fmt

.PHONY: lint

lint: ## Run Clippy linter
	cargo clippy -- -D warnings

.PHONY: test

test: ## Run tests (if any)
	cargo test

.PHONY: env-example

env-example: ## Copy .env.example to .env
	@if [ -f ".env" ]; then \
		echo "$(YELLOW).env already exists. Skipping.$(RESET)"; \
	else \
		cp .env.example .env; \
		echo "$(GREEN)✓ Created .env from .env.example$(RESET)"; \
		echo "  → Edit $(BLUE).env$(RESET) and configure your API keys."; \
	fi
