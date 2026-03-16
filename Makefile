WASM_TARGET = wasm32-wasip1
WASM_BIN    = target/$(WASM_TARGET)/release-small/sgleam.wasm
WEB_DIR     = web
DIST_DIR    = dist

DIST_FILES = \
	$(DIST_DIR)/sgleam.wasm \
	$(DIST_DIR)/index.html \
	$(DIST_DIR)/sgleam.js \
	$(DIST_DIR)/worker.js \
	$(DIST_DIR)/server.py

.PHONY: all serve test test-web test-rs check clean

all: $(DIST_FILES)

serve: $(DIST_FILES)
	( timeout 3 bash -c 'until curl -s http://localhost:8000 > /dev/null; do sleep 0.5; done; xdg-open http://localhost:8000' ) & \
	cd dist && python server.py

# WASM binary

$(DIST_DIR)/sgleam.wasm: $(WASM_BIN) | $(DIST_DIR)
	wasm-opt -Oz $< -o $@

RUST_SRCS = Cargo.toml \
	$(wildcard sgleam-core/src/*.rs) \
	$(wildcard cli/src/*.rs) \
	$(wildcard wasm/src/*.rs)

$(WASM_BIN): $(RUST_SRCS)
	cargo build -p sgleam-wasm --target $(WASM_TARGET) --profile release-small

# TypeScript compilation

$(DIST_DIR)/worker.js: $(WEB_DIR)/worker.ts $(WEB_DIR)/worker_channel.ts $(WEB_DIR)/ui_channel.ts $(WEB_DIR)/env.ts $(WEB_DIR)/wasi.ts | $(DIST_DIR)
	deno bundle $(WEB_DIR)/worker.ts -o $@

$(DIST_DIR)/sgleam.js: $(WEB_DIR)/ui.ts $(WEB_DIR)/ui_channel.ts $(WEB_DIR)/ansi.ts | $(DIST_DIR)
	deno bundle $(WEB_DIR)/ui.ts -o $@

# Static web files

$(DIST_DIR)/index.html: $(WEB_DIR)/sgleam.html | $(DIST_DIR)
	cp $< $@

$(DIST_DIR)/test.js: $(WEB_DIR)/test.ts $(WEB_DIR)/ui_channel.ts | $(DIST_DIR)
	deno bundle $(WEB_DIR)/test.ts -o $@

# Tests

test: test-rs test-web

test-rs:
	cargo test

test-web: $(DIST_DIR)/sgleam.wasm $(DIST_DIR)/worker.js $(DIST_DIR)/test.js
	deno test --allow-read $(WEB_DIR)/channel_test.ts
	deno test $(WEB_DIR)/ansi_test.ts
	deno test --allow-read $(DIST_DIR)/test.js

# Check

check:
	cargo clippy -- -D warnings
	cargo fmt -- --check
	deno fmt --check

# Utility

$(DIST_DIR):
	mkdir -p $@

clean:
	rm -rf $(DIST_DIR)
