SHELL := bash
.SHELLFLAGS := -eu -o pipefail -c

RUSTUP       ?= rustup
RUSTUP_TOOLCHAIN ?= stable
CARGO        := $(shell if command -v $(RUSTUP) >/dev/null 2>&1 && $(RUSTUP) which cargo >/dev/null 2>&1; then $(RUSTUP) which cargo; else command -v cargo; fi)
RUSTC        := $(shell if command -v $(RUSTUP) >/dev/null 2>&1 && $(RUSTUP) which rustc >/dev/null 2>&1; then $(RUSTUP) which rustc; else command -v rustc; fi)
RUSTDOC      := $(shell if command -v $(RUSTUP) >/dev/null 2>&1 && $(RUSTUP) which rustdoc >/dev/null 2>&1; then $(RUSTUP) which rustdoc; else command -v rustdoc; fi)
CARGO_ENV    := RUSTC="$(RUSTC)" RUSTDOC="$(RUSTDOC)"
APP          := gh-inbox
DOCKER       ?= docker
LINUX_BUILD_IMAGE ?= rust:1.94-bookworm
DOCKER_UID   ?= $(shell id -u)
DOCKER_GID   ?= $(shell id -g)
BINDIR       := bin
DISTDIR      := dist
VERSION      ?= $(shell git describe --tags --always --dirty 2>/dev/null || git rev-parse --short=12 HEAD 2>/dev/null || echo dev)
RELEASE_TAG  ?= $(VERSION)
MAIN_TAG     ?= main-$(shell git rev-parse --short=12 HEAD 2>/dev/null || echo dev)
DARWIN_ARCHS ?= amd64 arm64
LINUX_ARCHS  ?= amd64 arm64
RUST_TARGETS := x86_64-apple-darwin aarch64-apple-darwin

DARWIN_amd64_TARGET := x86_64-apple-darwin
DARWIN_amd64_SUFFIX := darwin-amd64
DARWIN_arm64_TARGET := aarch64-apple-darwin
DARWIN_arm64_SUFFIX := darwin-arm64

LINUX_amd64_PLATFORM := linux/amd64
LINUX_amd64_SUFFIX := linux-amd64
LINUX_arm64_PLATFORM := linux/arm64
LINUX_arm64_SUFFIX := linux-arm64

all: help

##@ Development

.PHONY: build
build: ## Build the host binary into bin/
	@echo "Building $(APP) for the host platform"
	@mkdir -p $(BINDIR)
	@$(CARGO_ENV) $(CARGO) build --release
	@cp target/release/$(APP) $(BINDIR)/$(APP)
	@chmod +x $(BINDIR)/$(APP)
	@echo "Wrote $(BINDIR)/$(APP)"

.PHONY: fmt
fmt: ## Format the Rust sources
	@$(CARGO_ENV) $(CARGO) fmt --all

.PHONY: fmt-check
fmt-check: ## Verify formatting without changing files
	@$(CARGO_ENV) $(CARGO) fmt --all --check

.PHONY: lint
lint: ## Run clippy with warnings treated as errors
	@$(CARGO_ENV) $(CARGO) clippy --all-targets --all-features -- -D warnings

.PHONY: test
test: ## Run the unit test suite
	@$(CARGO_ENV) $(CARGO) test

.PHONY: docker-check
docker-check: ## Verify that Docker is available for Linux release builds
	@command -v $(DOCKER) >/dev/null 2>&1 || { \
		echo "Docker is required for Linux release builds" >&2; \
		exit 1; \
	}
	@$(DOCKER) info >/dev/null 2>&1 || { \
		echo "A running Docker daemon is required for Linux release builds" >&2; \
		exit 1; \
	}

define TARGET_RULE
.PHONY: target.$(1)
target.$(1): ## Install the Rust target $(1) with rustup
	@command -v rustup >/dev/null 2>&1 || { \
		echo "rustup is required to install cross-compilation targets" >&2; \
		exit 1; \
	}
	@rustup target add $(1)
endef
$(foreach target,$(RUST_TARGETS),$(eval $(call TARGET_RULE,$(target))))

define DARWIN_DIST_RULE
.PHONY: dist-darwin.$(1)
dist-darwin.$(1): target.$$(DARWIN_$(1)_TARGET) ## Build the $(1) Darwin release asset
	@echo "Building $(APP) for $$(DARWIN_$(1)_TARGET)"
	@mkdir -p $(DISTDIR)
	@$(CARGO_ENV) $(CARGO) build --release --target $$(DARWIN_$(1)_TARGET)
	@cp target/$$(DARWIN_$(1)_TARGET)/release/$(APP) $(DISTDIR)/$(APP)_$(RELEASE_TAG)_$$(DARWIN_$(1)_SUFFIX)
	@chmod +x $(DISTDIR)/$(APP)_$(RELEASE_TAG)_$$(DARWIN_$(1)_SUFFIX)
	@echo "Wrote $(DISTDIR)/$(APP)_$(RELEASE_TAG)_$$(DARWIN_$(1)_SUFFIX)"
endef
$(foreach arch,$(DARWIN_ARCHS),$(eval $(call DARWIN_DIST_RULE,$(arch))))

.PHONY: dist-darwin
dist-darwin: ## Build all precompiled Darwin release assets
	@mkdir -p $(DISTDIR)
	@for arch in $(DARWIN_ARCHS); do \
		$(MAKE) dist-darwin.$$arch RELEASE_TAG=$(RELEASE_TAG) || exit $$?; \
	done

define LINUX_DIST_RULE
.PHONY: dist-linux.$(1)
dist-linux.$(1): docker-check ## Build the $(1) Linux release asset via Docker
	@echo "Building $(APP) for $$(LINUX_$(1)_PLATFORM) via Docker"
	@mkdir -p $(DISTDIR) .cargo-linux/$(1) .home-linux/$(1)
	@$(DOCKER) run --rm \
		--platform $$(LINUX_$(1)_PLATFORM) \
		-u "$(DOCKER_UID):$(DOCKER_GID)" \
		-e HOME=/workspace/.home-linux/$(1) \
		-e CARGO_HOME=/workspace/.cargo-linux/$(1) \
		-e CARGO_TARGET_DIR=/workspace/target/linux-$(1) \
		-v "$(CURDIR):/workspace" \
		-w /workspace \
		$(LINUX_BUILD_IMAGE) \
		bash -c 'export PATH="/usr/local/cargo/bin:$$$$PATH"; cargo build --release && cp target/linux-$(1)/release/$(APP) dist/$(APP)_$(RELEASE_TAG)_$$(LINUX_$(1)_SUFFIX) && chmod +x dist/$(APP)_$(RELEASE_TAG)_$$(LINUX_$(1)_SUFFIX)'
	@echo "Wrote $(DISTDIR)/$(APP)_$(RELEASE_TAG)_$$(LINUX_$(1)_SUFFIX)"
endef
$(foreach arch,$(LINUX_ARCHS),$(eval $(call LINUX_DIST_RULE,$(arch))))

.PHONY: dist-linux
dist-linux: ## Build all precompiled Linux release assets
	@mkdir -p $(DISTDIR)
	@for arch in $(LINUX_ARCHS); do \
		$(MAKE) dist-linux.$$arch RELEASE_TAG=$(RELEASE_TAG) || exit $$?; \
	done

##@ Release

.PHONY: dist
dist: ## Build all precompiled release assets
	@rm -rf $(DISTDIR)
	@mkdir -p $(DISTDIR)
	@$(MAKE) dist-darwin RELEASE_TAG=$(RELEASE_TAG)
	@$(MAKE) dist-linux RELEASE_TAG=$(RELEASE_TAG)

.PHONY: publish-release
publish-release: ## Publish the assets already present in dist/ to GitHub Releases. Use TAG=v0.1.0.
	@if [ -z "$(TAG)" ]; then \
		echo "TAG is required. Example: make publish-release TAG=v0.1.0" >&2; \
		exit 1; \
	fi
	@if ! ls $(DISTDIR)/* >/dev/null 2>&1; then \
		echo "No release assets found in $(DISTDIR). Run make dist, make dist-darwin, or make dist-linux first." >&2; \
		exit 1; \
	fi
	@if gh release view "$(TAG)" >/dev/null 2>&1; then \
		echo "Uploading assets to existing release $(TAG)"; \
		gh release upload "$(TAG)" $(DISTDIR)/* --clobber; \
	else \
		echo "Creating release $(TAG)"; \
		gh release create "$(TAG)" $(DISTDIR)/* \
			--title "$(TAG)" \
			--notes "Release $(TAG)" \
			--target "$$(git rev-parse HEAD)"; \
	fi

.PHONY: publish-release-main
publish-release-main: ## Publish the assets already present in dist/ to the rolling main release
	@$(MAKE) publish-release TAG="$(MAIN_TAG)"

.PHONY: release
release: ## Build all precompiled release assets and publish them. Use TAG=v0.1.0.
	@if [ -z "$(TAG)" ]; then \
		echo "TAG is required. Example: make release TAG=v0.1.0" >&2; \
		exit 1; \
	fi
	@$(MAKE) dist RELEASE_TAG=$(TAG)
	@$(MAKE) publish-release TAG=$(TAG)

.PHONY: release-main
release-main: ## Build all precompiled release assets and publish the rolling main release
	@tag="$(MAIN_TAG)"; \
	$(MAKE) dist RELEASE_TAG="$$tag"; \
	$(MAKE) publish-release TAG="$$tag"

.PHONY: clean
clean: ## Remove build artifacts
	@echo "Cleaning build artifacts"
	@rm -rf $(BINDIR) $(DISTDIR) .cargo-linux .home-linux
	@$(CARGO) clean

##@ Help

.PHONY: help
help: ## Show this help message
	@awk 'BEGIN {FS = ":.*##"; section = ""} \
	/^[a-zA-Z0-9_.-]+:.*##/ { \
		if (section != "") printf "\n\033[1m%s\033[0m\n", section; \
		section = ""; \
		printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2; next \
	} \
	/^##@/ { section = substr($$0, 5); next }' $(MAKEFILE_LIST)
	@printf "\n\033[1mDarwin Architectures:\033[0m\n"
	@printf "  \033[36mamd64\033[0m -> %s\n" "$(DARWIN_amd64_TARGET)"
	@printf "  \033[36marm64\033[0m -> %s\n" "$(DARWIN_arm64_TARGET)"
	@printf "\n\033[1mLinux Architectures:\033[0m\n"
	@printf "  \033[36mamd64\033[0m -> %s\n" "$(LINUX_amd64_PLATFORM)"
	@printf "  \033[36marm64\033[0m -> %s\n" "$(LINUX_arm64_PLATFORM)"
	@printf "\n\033[1mExamples:\033[0m\n"
	@printf "  \033[36mmake build\033[0m\n"
	@printf "  \033[36mmake test\033[0m\n"
	@printf "  \033[36mmake dist-darwin RELEASE_TAG=main-%s\033[0m\n" "$$(git rev-parse --short=12 HEAD 2>/dev/null || echo dev)"
	@printf "  \033[36mmake dist-linux RELEASE_TAG=main-%s\033[0m\n" "$$(git rev-parse --short=12 HEAD 2>/dev/null || echo dev)"
	@printf "  \033[36mmake dist RELEASE_TAG=main-%s\033[0m\n" "$$(git rev-parse --short=12 HEAD 2>/dev/null || echo dev)"
	@printf "  \033[36mmake publish-release TAG=v0.1.0\033[0m\n"
	@printf "  \033[36mmake release TAG=v0.1.0\033[0m\n"
	@printf "  \033[36mmake release-main\033[0m\n"
