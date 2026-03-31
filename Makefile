WASM_TARGET = wasm32-wasip1

.PHONY: wasm test test-rs test-wasm check docs clean

wasm:
	cargo build -p sgleam-wasm --target $(WASM_TARGET) --profile release-small

test: test-rs test-wasm

test-rs:
	cargo test
	cargo test -p engine --features resvg
	cargo test -p tests

test-wasm: wasm
	deno test --allow-read wasm/tests/

check:
	cargo clippy -- -D warnings
	cargo clippy --features resvg -- -D warnings
	cargo clippy --target $(WASM_TARGET) -p sgleam-wasm -- -D warnings
	cargo fmt -- --check
	deno fmt --check wasm/tests/

docs:
	bash docs/build-docs.sh

clean:
	rm -rf dist/docs
