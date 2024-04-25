.DEFAULT_GOAL := all


PROFILE := release
debug ?=
target ?= $(shell rustc -vV | sed -n 's|host: ||p')
arch = $(shell uname -m)

ifdef debug
  cargo_build_opts :=
  target_type := debug
else
  cargo_build_opts := --release
  target_type = release
endif

ifneq ($(target),)
    cargo_build_opts += --target $(target)
endif

export PYO3_CONFIG_FILE = $(CURDIR)/querent/build/pyo3-build-config-file-$(target).txt
$(info PYO3_CONFIG_FILE = $(PYO3_CONFIG_FILE))

# Determine the operating system
UNAME := $(shell uname)

PROTOC := protoc
 ifeq ($(UNAME), Darwin)
	PROTOC = /opt/homebrew/opt/protobuf@21/bin/protoc
endif

CBINDGEN=${HOME}/.cargo/bin/cbindgen

.PHONY: check
check:
	@echo "Running Cargo check..."
	@cargo check --all --all-features --all-targets

.PHONY: test
test:
	@echo "Running Cargo test..."
	@cargo test

.PHONY: clippy
clippy:
	@echo "Running Cargo clippy..."
	@cargo clippy --all --all-features --all-targets -- -D warnings

.PHONY: deny
deny:
	@echo "Running Cargo deny..."
	@cargo deny check -c .cargo-deny.toml

.PHONY: format
format:
	@echo "Running Cargo fmt..."
	@rustup component add rustfmt --toolchain nightly
	@cargo +nightly fmt --all
format-check:
	@echo "Running Cargo fmt..."
	@rustup component add rustfmt --toolchain nightly
	@cargo +nightly fmt --all -- --check

.PHONY: build
build:
	@echo "Running Cargo build..."
	@env -u PYO3_CONFIG_FILE cargo build $(cargo_build_opts) --all-features

.PHONY: doc
doc:
	@echo "Running Cargo doc..."
	@RUSTDOCFLAGS="--enable-index-page --check -Zunstable-options" cargo doc --no-deps --all-features

.PHONY: clean
clean:
	@echo "Running Cargo clean..."
	@cargo clean

.PHONY: all
all: check test clippy deny format build doc

.PHONY: build-ui
build-ui:
	NODE_ENV=production cd ../ui && $(MAKE) install build

.PHONY: archive
archive:
	@echo "Archiving release binary & assets"
	@mkdir -p "./querent-${BINARY_VERSION}/config"
	@mkdir -p "./querent-${BINARY_VERSION}/querent_data"
	@cp ../config/querent.config.yaml "./querent-${BINARY_VERSION}/config"
	@cp ../config/querent.config.docker.yaml "./querent-${BINARY_VERSION}/config"
	@cp ../LICENSE ./querent-${BINARY_VERSION}
	@cp "${BINARY_FILE}" "./querent-${BINARY_VERSION}"
	@tar -czf "${ARCHIVE_NAME}.tar.gz" "./querent-${BINARY_VERSION}"
	@rm -rf "./querent-${BINARY_VERSION}"
