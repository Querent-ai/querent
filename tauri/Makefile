.PHONY:check
check-rian:
	@echo "Running cargo check"
	@cd rian/src-tauri && cargo check

.PHONY:test
test-rian:
	@echo "Running cargo test"
	@cd rian/src-tauri && cargo test

.PHONY:build-rian
build:
	@echo "Running cargo build"
	@cd rian/src-tauri && cargo build

.PHONY:run-rian
run:
	@echo "Running tauri dev"
	@cd rian/src-tauri && cargo tauri dev

.PHONY:format-rian
format:
	@echo "Running cargo fmt"
	@cd rian/src-tauri && cargo +nightly fmt

.PHONY:clean-rian
clean:
	@echo "Running cargo clean"
	@cd rian/src-tauri && cargo clean