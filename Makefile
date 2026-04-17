WASM_TARGET = wasm32-wasip1

.PHONY: release wasm test test-rs test-wasm test-wasm-cli check docs clean

release:
	cargo build --release
	objcopy --remove-section=.eh_frame --remove-section=.eh_frame_hdr target/release/sgleam

wasm:
	cargo build -p wasm --target $(WASM_TARGET) --profile release-small
	wasm-opt -Oz --enable-bulk-memory --enable-mutable-globals --enable-sign-ext --enable-nontrapping-float-to-int target/$(WASM_TARGET)/release-small/sgleam.wasm -o target/$(WASM_TARGET)/release-small/sgleam.wasm

test: test-rs test-wasm test-wasm-cli

test-rs:
	cargo test
	cargo test -p engine --features resvg
	cargo test -p tests

test-wasm: wasm
	deno test --allow-read wasm/tests/

# Runs the CLI integration tests against both the native binary and the Deno
# WASM wrapper, asserting both backends produce identical output. Requires
# the wasm32-wasip1 target and Deno. CI should enable this only on one
# platform (e.g., Linux) to avoid redundant WASM runs across the matrix.
test-wasm-cli: wasm
	cargo test -p sgleam --features wasm-backend --test cli

check:
	cargo clippy -- -D warnings
	cargo clippy --features resvg -- -D warnings
	cargo clippy --target $(WASM_TARGET) -p wasm -- -D warnings
	cargo fmt -- --check
	cargo run -- format --check lib/sgleam/*.gleam
	deno fmt --check wasm/tests/ wasm/sgleam.ts

docs:
	bash docs/build-docs.sh

clean:
	rm -rf dist/docs
