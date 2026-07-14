# Makefile for the iLEAP skill microsite (docs/adr/0010).
#
# Installs a pinned Zola binary into ./bin (no system-wide install, no Node,
# no Rust toolchain needed) and builds/serves the site under ./site.
#
#   make site-serve     # install zola if needed, run the dev server
#   make site-build     # install zola if needed, build to site/public
#   make site-check     # validate the site (links, etc.)
#   make site-clean     # remove the built output
#   make tools-clean    # remove the downloaded zola binary
#
# Override the version with:  make site-build ZOLA_VERSION=0.20.0

ZOLA_VERSION ?= 0.19.2
SITE_DIR     := site
BIN_DIR      := $(CURDIR)/bin
ZOLA         := $(BIN_DIR)/zola

# --- Resolve the Zola release asset for this OS/arch -------------------------
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)

SKILL_CLI_BIN_DIR := $(CURDIR)/ileap/bin
SKILL_CLI_BIN := $(SKILL_CLI_BIN_DIR)/ileap-$(UNAME_S)-$(UNAME_M)

ifeq ($(UNAME_S),Darwin)
  ifeq ($(UNAME_M),arm64)
    ZOLA_TARGET := aarch64-apple-darwin
  else
    ZOLA_TARGET := x86_64-apple-darwin
  endif
else ifeq ($(UNAME_S),Linux)
  ifeq ($(UNAME_M),aarch64)
    ZOLA_TARGET := aarch64-unknown-linux-gnu
  else
    ZOLA_TARGET := x86_64-unknown-linux-gnu
  endif
endif

ZOLA_URL := https://github.com/getzola/zola/releases/download/v$(ZOLA_VERSION)/zola-v$(ZOLA_VERSION)-$(ZOLA_TARGET).tar.gz

# --- skills-ref (repo-local, no global/$$HOME install) -----------------------
# Installed into a venv under ./bin so it is covered by .gitignore and
# tools-clean. Pin a specific commit/tag with: make ci-skill SKILLS_REF_REV=...
SKILLS_REF_REV  ?= main
SKILLS_REF_VENV := $(BIN_DIR)/skills-ref-venv
SKILLS_REF      := $(SKILLS_REF_VENV)/bin/skills-ref
SKILLS_REF_URL  := git+https://github.com/agentskills/agentskills.git@$(SKILLS_REF_REV)\#subdirectory=skills-ref

.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help
	@echo "iLEAP skill — make targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

# --- Install Zola locally ----------------------------------------------------
.PHONY: install-zola
install-zola: $(ZOLA) ## Download the pinned Zola binary into ./bin

$(ZOLA):
	@if [ -z "$(ZOLA_TARGET)" ]; then \
		echo "Unsupported platform: $(UNAME_S)/$(UNAME_M). Install zola manually and put it on PATH." >&2; \
		exit 1; \
	fi
	@command -v curl >/dev/null 2>&1 || { echo "curl is required to download zola." >&2; exit 1; }
	@mkdir -p $(BIN_DIR)
	@echo "Downloading zola $(ZOLA_VERSION) ($(ZOLA_TARGET))..."
	@curl -fsSL "$(ZOLA_URL)" | tar -xz -C $(BIN_DIR) zola
	@chmod +x $(ZOLA)
	@$(ZOLA) --version

# --- Install skills-ref locally ------------------------------------------------
.PHONY: install-skills-ref
install-skills-ref: $(SKILLS_REF)

$(SKILLS_REF):
	@command -v python3 >/dev/null 2>&1 || { echo "python3 is required to install skills-ref." >&2; exit 1; }
	@echo "Installing skills-ref ($(SKILLS_REF_REV)) into $(SKILLS_REF_VENV)..."
	python3 -m venv $(SKILLS_REF_VENV)
	$(SKILLS_REF_VENV)/bin/pip install --quiet "$(SKILLS_REF_URL)"

# --- Build / serve the site --------------------------------------------------
# Zola bakes base_url into absolute URLs at build time, so the static value in
# site/config.toml would break Vercel preview deploys (unique host per deploy).
# make reads the environment directly: on a Vercel preview/dev build we override
# base_url with this deploy's own host ($VERCEL_URL); production and local builds
# use site/config.toml. Override manually any time with: make site-build BASE_URL=...
ifneq ($(VERCEL_URL),)
ifneq ($(VERCEL_ENV),production)
BASE_URL ?= https://$(VERCEL_URL)
endif
endif

ZOLA_BUILD_FLAGS := $(if $(BASE_URL),--base-url $(BASE_URL),)

.PHONY: site-build
site-build: $(ZOLA) ## Build to site/public (override host with BASE_URL=...)
	cd $(SITE_DIR) && $(ZOLA) build $(ZOLA_BUILD_FLAGS)
	@echo "Built into $(SITE_DIR)/public$(if $(BASE_URL), (base-url $(BASE_URL)),)"

.PHONY: site-serve
site-serve: $(ZOLA) ## Run the local dev server (http://127.0.0.1:1111)
	cd $(SITE_DIR) && $(ZOLA) serve

.PHONY: site-check
site-check: $(ZOLA) ## Validate the site (internal links, etc.)
	cd $(SITE_DIR) && $(ZOLA) check

# --- Cleanup -----------------------------------------------------------------
.PHONY: site-clean
site-clean: ## Remove the built output (site/public)
	rm -rf $(SITE_DIR)/public

.PHONY: tools-clean
tools-clean: ## Remove downloaded tools (./bin: zola, skills-ref venv)
	rm -rf $(BIN_DIR)
	rm -rf $(SKILLS_REF_VENV)

.PHONY: distclean
distclean: site-clean tools-clean rust-clean skill-clean ## Remove built output and downloaded tools

.PHONY: ci
ci: ## run *all* CI actions locally
	@echo "Running CI checks..."
	@$(MAKE) ci-cli ci-skill site-build site-check

.PHONY: ci-skill
ci-skill: $(SKILLS_REF) ## Validate the ileap skill with a repo-local skills-ref
	@echo "Running skill checks..."
	$(SKILLS_REF) validate ./ileap

.PHONY: ci-cli
ci-cli: ## Perform all checks on the CLI (clippy, fmt, test, build)
	cargo clippy --all-targets --all-features -- -D warnings
	cargo fmt --all -- --check
	cargo test --all-features
	cargo build --release

.PHONY: package
package: $(SKILL_CLI_BIN) ## Build a (local) skill release
	scripts/package-skill.sh

.PHONY: skill-clean
skill-clean: ## Remove the built skill release
	rm -rf ileap/bin

$(SKILL_CLI_BIN):
	@echo "Building the iLEAP CLI for $(UNAME_S)/$(UNAME_M)..."
	cargo build --release --locked
	mkdir -p $(SKILL_CLI_BIN_DIR)
	cp target/release/ileap $@

rust-clean: ## Remove the built Rust artifacts
	rm -rf target