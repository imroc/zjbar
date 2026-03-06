PLUGIN_DIR := $(HOME)/.config/zellij/plugins
LAYOUT_DIR := $(HOME)/.config/zellij/layouts
WASM       := target/wasm32-wasip1/release/zjbar.wasm
TAG        := $(shell git describe --tags --exact-match 2>/dev/null)

.PHONY: build install clean release

install: build
	@mkdir -p $(PLUGIN_DIR) $(LAYOUT_DIR)
	cp $(WASM) $(PLUGIN_DIR)/zjbar.wasm
	cp layout.kdl $(LAYOUT_DIR)/zjbar.kdl
	cp layout.swap.kdl $(LAYOUT_DIR)/zjbar.swap.kdl
	@echo "Installed plugin and layouts."

build:
	cargo build --release

clean:
	cargo clean

release: build
	@if [ -z "$(TAG)" ]; then \
		echo "Error: HEAD has no tag. Tag first with: git tag vX.Y.Z"; \
		exit 1; \
	fi
	gh release create $(TAG) $(WASM) --title "$(TAG)" --generate-notes
