WASM_TARGET = wasm32-wasip1

.PHONY: release wasm test test-rs test-wasm check docs clean

release:
	cargo build --release
	objcopy --remove-section=.eh_frame --remove-section=.eh_frame_hdr target/release/sgleam

wasm:
	cargo build -p wasm --target $(WASM_TARGET) --profile release-small
	wasm-opt -Oz --enable-bulk-memory --enable-mutable-globals --enable-sign-ext --enable-nontrapping-float-to-int target/$(WASM_TARGET)/release-small/sgleam.wasm -o target/$(WASM_TARGET)/release-small/sgleam.wasm

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
	cargo clippy --target $(WASM_TARGET) -p wasm -- -D warnings
	cargo fmt -- --check
	cargo run -- format --check lib/sgleam/*.gleam
	deno fmt --check wasm/tests/

docs:
	bash docs/build-docs.sh

clean:
	rm -rf dist/docs
