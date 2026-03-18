#!/bin/bash
set -e

mkdir -p dist/docs

# Generate SVG images
cargo run -q -- docs/gen_images.gleam > docs/images.svg

build() {
  local output=$1; shift
  pandoc "$@" \
    --standalone --toc \
    --highlight-style=docs/one-light.theme \
    --syntax-definition=docs/gleam.xml \
    --syntax-definition=docs/gleam-repl.xml \
    --lua-filter=docs/gleam-inline.lua \
    --css=docs/style.css \
    --embed-resources \
    --include-after-body=docs/theme-toggle.html \
    --metadata author="Marco A L Barbosa" \
    --metadata date="2026" \
    -o "$output"
  echo "Generated $output"
}

# Portuguese
build dist/docs/cli.pt-br.html \
  docs/pt-br/index-cli.md docs/pt-br/cli.md \
  --metadata title="Guia do Sgleam - CLI"

build dist/docs/web.pt-br.html \
  docs/pt-br/index-web.md docs/pt-br/web.md docs/pt-br/imagens.md docs/pt-br/mundo.md \
  --lua-filter=docs/inject-images.lua \
  --metadata title="Guia do Sgleam - Web"

# English
build dist/docs/cli.en.html \
  docs/en/index-cli.md docs/en/cli.md \
  --metadata title="Sgleam Guide - CLI"

build dist/docs/web.en.html \
  docs/en/index-web.md docs/en/web.md docs/en/images.md docs/en/world.md \
  --lua-filter=docs/inject-images.lua \
  --metadata title="Sgleam Guide - Web"
