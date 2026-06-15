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

.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help
	@echo "iLEAP microsite — make targets:"
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
tools-clean: ## Remove the downloaded zola binary (./bin)
	rm -rf $(BIN_DIR)

.PHONY: distclean
distclean: site-clean tools-clean ## Remove built output and downloaded tools
