WASM_TARGET = wasm32-wasip1

.PHONY: release release-min release-nightly wasm test test-rs test-wasm test-wasm-cli check docs clean

NIGHTLY_BIN = target/min-nightly/x86_64-unknown-linux-gnu/release/sgleam

release:
	cargo build --release
	objcopy --remove-section=.eh_frame --remove-section=.eh_frame_hdr target/release/sgleam

# Build mínimo: strip debug info + remove unwind tables + disable PIE.
# Usa `cargo rustc` para aplicar -no-pie só ao binário final (proc-macros
# continuam sendo dylibs PIC). Target-dir dedicado evita invalidar o cache
# do `release` normal.
release-min:
	cargo rustc --release -p sgleam --bin sgleam --target-dir target/min -- \
	    -C relocation-model=static -C link-arg=-no-pie
	strip --strip-all target/min/release/sgleam
	objcopy --remove-section=.eh_frame --remove-section=.eh_frame_hdr target/min/release/sgleam
	@echo "Artefato em target/min/release/sgleam"
	@ls -lh target/min/release/sgleam

# Como release-min, mas recompila std com optimize_for_size (requer nightly
# + `rustup component add rust-src --toolchain nightly`). Reduz ~500 KiB
# sobre release-min ao custo de ~1:30 na primeira build. Panic handler e
# mensagens permanecem (só removemos debug/unwind/PIE, não panic info).
release-nightly:
	cargo +nightly rustc --release \
	    -Z build-std=std,panic_abort \
	    -Z build-std-features=optimize_for_size \
	    --target x86_64-unknown-linux-gnu \
	    -p sgleam --bin sgleam --target-dir target/min-nightly -- \
	    -C relocation-model=static -C link-arg=-no-pie
	strip --strip-all $(NIGHTLY_BIN)
	objcopy --remove-section=.eh_frame --remove-section=.eh_frame_hdr $(NIGHTLY_BIN)
	@echo "Artefato em $(NIGHTLY_BIN)"
	@ls -lh $(NIGHTLY_BIN)

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
