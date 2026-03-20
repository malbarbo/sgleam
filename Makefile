WASM_TARGET = wasm32-wasip1
WASM_BIN    = target/$(WASM_TARGET)/release-small/sgleam.wasm
WEB_DIR     = web
DIST_DIR    = dist
BUILD_DIR   = build

DIST_FILES = \
	$(DIST_DIR)/sgleam.wasm \
	$(DIST_DIR)/index.html \
	$(DIST_DIR)/player.html \
	$(DIST_DIR)/manifest.json \
	$(DIST_DIR)/service-worker.js \
	$(DIST_DIR)/icon-192.svg \
	$(DIST_DIR)/icon-512.svg \
	$(DIST_DIR)/server.py

.PHONY: all serve test test-web test-rs check clean docs

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

# TypeScript compilation (intermediate build files)

$(BUILD_DIR)/worker.js: $(WEB_DIR)/worker.ts $(WEB_DIR)/worker_channel.ts $(WEB_DIR)/ui_channel.ts $(WEB_DIR)/env.ts $(WEB_DIR)/wasi.ts | $(BUILD_DIR)
	deno bundle $(WEB_DIR)/worker.ts -o $@

$(BUILD_DIR)/sgleam.js: $(WEB_DIR)/ui.ts $(WEB_DIR)/ui_channel.ts $(WEB_DIR)/ansi.ts | $(BUILD_DIR)
	deno bundle $(WEB_DIR)/ui.ts -o $@

$(BUILD_DIR)/player.js: $(WEB_DIR)/player.ts $(WEB_DIR)/ui_channel.ts | $(BUILD_DIR)
	deno bundle $(WEB_DIR)/player.ts -o $@

# Download CodeFlask

$(BUILD_DIR)/codeflask.min.js: | $(BUILD_DIR)
	curl -sL "https://unpkg.com/codeflask/build/codeflask.min.js" -o $@

# Inline everything into a single HTML file

$(DIST_DIR)/index.html $(DIST_DIR)/player.html: $(WEB_DIR)/sgleam.html $(WEB_DIR)/player.html $(BUILD_DIR)/sgleam.js $(BUILD_DIR)/player.js $(BUILD_DIR)/worker.js $(BUILD_DIR)/codeflask.min.js $(WEB_DIR)/inline.ts | $(DIST_DIR)
	cp $(BUILD_DIR)/sgleam.js $(DIST_DIR)/sgleam.js
	cp $(BUILD_DIR)/player.js $(DIST_DIR)/player.js
	cp $(BUILD_DIR)/worker.js $(DIST_DIR)/worker.js
	deno run --allow-read --allow-write $(WEB_DIR)/inline.ts
	rm $(DIST_DIR)/sgleam.js $(DIST_DIR)/player.js $(DIST_DIR)/worker.js

# Static web files

$(DIST_DIR)/server.py: $(WEB_DIR)/server.py | $(DIST_DIR)
	cp $< $@

$(DIST_DIR)/manifest.json: $(WEB_DIR)/manifest.json | $(DIST_DIR)
	cp $< $@

$(DIST_DIR)/service-worker.js: $(WEB_DIR)/service-worker.js | $(DIST_DIR)
	cp $< $@

$(DIST_DIR)/icon-192.svg: $(WEB_DIR)/icon-192.svg | $(DIST_DIR)
	cp $< $@

$(DIST_DIR)/icon-512.svg: $(WEB_DIR)/icon-512.svg | $(DIST_DIR)
	cp $< $@

# Tests

$(DIST_DIR)/test.js: $(WEB_DIR)/test.ts $(WEB_DIR)/ui_channel.ts | $(DIST_DIR)
	deno bundle $(WEB_DIR)/test.ts -o $@

test: test-rs test-web

test-rs:
	cargo test
	cargo test -p sgleam-core-tests

test-web: $(DIST_DIR)/sgleam.wasm $(DIST_DIR)/test.js $(BUILD_DIR)/worker.js
	cp $(BUILD_DIR)/worker.js $(DIST_DIR)/worker.js
	deno test --allow-read $(WEB_DIR)/channel_test.ts
	deno test $(WEB_DIR)/ansi_test.ts
	deno test $(WEB_DIR)/dirty_test.ts
	deno test $(WEB_DIR)/wasi_test.ts
	deno test --allow-read $(DIST_DIR)/test.js

# Check

check:
	cargo clippy -- -D warnings
	cargo clippy --target $(WASM_TARGET) -p sgleam-wasm -- -D warnings
	cargo fmt -- --check
	deno fmt --check

# Utility

$(DIST_DIR):
	mkdir -p $@

$(BUILD_DIR):
	mkdir -p $@

docs:
	bash docs/build-docs.sh

clean:
	rm -rf $(DIST_DIR) $(BUILD_DIR)
