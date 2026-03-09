WASM_TARGET = wasm32-wasip1
WASM_BIN    = target/$(WASM_TARGET)/release/sgleam.wasm
WEB_DIR     = web
DIST_DIR    = dist

DIST_FILES = \
	$(DIST_DIR)/sgleam.wasm \
	$(DIST_DIR)/index.html \
	$(DIST_DIR)/sgleam.js \
	$(DIST_DIR)/repl.js \
	$(DIST_DIR)/channel.js \
	$(DIST_DIR)/server.py

.PHONY: all serve test test-wasm clean

all: $(DIST_FILES)

serve: $(DIST_FILES)
	( timeout 3 bash -c 'until curl -s http://localhost:8000 > /dev/null; do sleep 0.5; done; xdg-open http://localhost:8000' ) & \
	cd dist && python server.py

# WASM binary

$(DIST_DIR)/sgleam.wasm: $(WASM_BIN) | $(DIST_DIR)
	cp $< $@

$(WASM_BIN):
	cargo build --target $(WASM_TARGET) --release

# TypeScript compilation

$(DIST_DIR)/channel.js: $(WEB_DIR)/channel.ts | $(DIST_DIR)
	deno bundle $< -o $@

$(DIST_DIR)/repl.js: $(WEB_DIR)/repl.ts $(WEB_DIR)/channel.ts | $(DIST_DIR)
	deno bundle $(WEB_DIR)/repl.ts -o $@

# Static web files

$(DIST_DIR)/index.html: $(WEB_DIR)/sgleam.html | $(DIST_DIR)
	cp $< $@

$(DIST_DIR)/sgleam.js: $(WEB_DIR)/sgleam.js | $(DIST_DIR)
	cp $< $@

$(DIST_DIR)/test.js: $(WEB_DIR)/test.js | $(DIST_DIR)
	cp $< $@

# Tests

test:
	deno test --allow-read $(WEB_DIR)/channel_test.ts

test-wasm: $(DIST_DIR)/sgleam.wasm $(DIST_DIR)/repl.js \
           $(DIST_DIR)/channel.js $(DIST_DIR)/test.js
	deno test --allow-read $(DIST_DIR)/test.js

# Utility

$(DIST_DIR):
	mkdir -p $@

clean:
	rm -rf $(DIST_DIR)
