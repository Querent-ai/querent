.PHONY:check
check:
	@echo "Running cargo check"
	@cd src-tauri && cargo check

.PHONY:test
test:
	@echo "Running cargo test"
	@cd src-tauri && cargo test

.PHONY:build
build:
	@echo "Running cargo build"
	@cd src-tauri && cargo build

.PHONY:run
run:
	@echo "Running tauri dev"
	@cd src-tauri && cargo tauri dev

.PHONY:format
format:
	@echo "Running cargo fmt"
	@cd src-tauri && cargo +nightly fmt