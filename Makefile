# musa — repository tasks.
#
# `make` on its own lists what there is to do. Everything here is a thin,
# discoverable name for a command that already works; nothing is hidden behind
# a script, so any target can also be typed out by hand.

UI      := apps/musa-desktop/ui
SHELL_D := apps/musa-desktop/src-tauri
TAURI   := $(CURDIR)/$(UI)/node_modules/.bin/tauri
NPM     := npm --prefix $(UI)
CARGO   := cargo

# The example a target uses when you do not name one: `make play FILE=...`.
FILE ?= examples/glass-mountain.musa
# Where `make render` writes. WAV by default; `make render TO=mei` prints MEI.
TO   ?= wav
OUT  ?= target/out.$(TO)

.DEFAULT_GOAL := help

## ---------------------------------------------------------------- running --

.PHONY: desktop
desktop: node_modules ## Start the desktop app (Tauri shell + Svelte UI, hot reload)
	cd $(SHELL_D) && $(TAURI) dev

.PHONY: ui
ui: node_modules ## Start the UI alone in a browser, against the stubbed shell
	$(NPM) run dev

.PHONY: check-file
check-file: ## Compile one .musa file and print its diagnostics (FILE=...)
	$(CARGO) run -q -p musa-cli -- check $(FILE)

.PHONY: render
render: ## Render FILE to TO (wav|midi|mei|lilypond|performance|plan) at OUT
	@mkdir -p $(dir $(OUT))
	$(CARGO) run -q -p musa-cli -- render $(FILE) --to $(TO) -o $(OUT)
	@echo "wrote $(OUT)"

.PHONY: play
play: ## Play FILE through the audio engine
	$(CARGO) run -q -p musa-cli -- play $(FILE)

## ---------------------------------------------------------------- building --

.PHONY: build
build: ## Build the whole Rust workspace (debug)
	$(CARGO) build --workspace

.PHONY: release
release: node_modules ## Bundle the desktop app for distribution
	cd $(SHELL_D) && $(TAURI) build

node_modules: $(UI)/package-lock.json ## Install the UI's dependencies
	$(NPM) ci
	@touch $(UI)/node_modules

.PHONY: setup
setup: node_modules ## Install everything a fresh checkout needs
	$(NPM) exec -- playwright install chromium

## ----------------------------------------------------------------- testing --

.PHONY: test
test: test-rust test-ui ## Run every test, Rust and UI

.PHONY: test-rust
test-rust: ## Run the Rust test suite (nextest if present, else cargo test)
	@if command -v cargo-nextest >/dev/null 2>&1; then \
		$(CARGO) nextest run --workspace; \
	else \
		$(CARGO) test --workspace; \
	fi

.PHONY: test-ui
test-ui: node_modules ## Run the UI unit tests and the Playwright screen tests
	$(NPM) test

.PHONY: test-unit
test-unit: node_modules ## Run the UI unit tests only (fast)
	$(NPM) run test:unit

## ------------------------------------------------------------------- gates --

.PHONY: fmt
fmt: ## Format Rust sources (and TOML, if taplo is installed) in place
	$(CARGO) fmt --all
	@if command -v taplo >/dev/null 2>&1; then taplo fmt; fi

.PHONY: lint
lint: ## Clippy over the workspace, warnings denied
	$(CARGO) clippy --workspace --all-targets -- -D warnings

.PHONY: typecheck
typecheck: node_modules ## Svelte + TypeScript check of the UI
	$(NPM) run check

.PHONY: deny
deny: ## Audit dependencies (licences, advisories); no-op if cargo-deny is absent
	@if command -v cargo-deny >/dev/null 2>&1; then $(CARGO) deny check; \
	else echo "cargo-deny not installed — skipping"; fi

.PHONY: verify
verify: ## Everything CI would check, in the order that fails fastest
	$(CARGO) fmt --all --check
	$(MAKE) lint
	$(MAKE) test-rust
	$(MAKE) typecheck
	$(MAKE) test-ui
	$(MAKE) deny

## ----------------------------------------------------------- housekeeping --

.PHONY: snapshots
snapshots: node_modules ## Re-record the golden snapshots after a deliberate change
	INSTA_UPDATE=always $(CARGO) test --workspace
	UPDATE_UI_FIXTURES=1 $(CARGO) test -p musa-project
	UPDATE_UI_FIXTURES=1 $(CARGO) test -p musa-desktop
	$(NPM) run screens:update
	@find . -name '*.snap.new' -delete

.PHONY: clean
clean: ## Remove build output (Rust target, UI dist, Playwright results)
	$(CARGO) clean
	rm -rf $(UI)/dist $(UI)/test-results

.PHONY: help
help: ## List these targets
	@echo "musa — notation-first music language and workbench"
	@echo
	@grep -hE '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[1m%-12s\033[0m %s\n", $$1, $$2}'
	@echo
	@echo "  Variables: FILE=$(FILE) TO=$(TO) OUT=$(OUT)"
