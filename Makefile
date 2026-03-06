PLUGIN_DIR := $(HOME)/.config/zellij/plugins
LAYOUT_DIR := $(HOME)/.config/zellij/layouts
WASM       := target/wasm32-wasip1/release/zjbar.wasm

.PHONY: build install clean

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
