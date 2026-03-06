PLUGIN_DIR := $(HOME)/.config/zellij/plugins
LAYOUT_DIR := $(HOME)/.config/zellij/layouts
WASM       := target/wasm32-wasip1/release/zjbar.wasm
TAG        := $(shell git describe --tags --exact-match 2>/dev/null)

.PHONY: build install install-layouts install-hooks uninstall-hooks uninstall clean release

build:
	cargo build --release
	@mkdir -p $(PLUGIN_DIR)
	cp $(WASM) $(PLUGIN_DIR)/zjbar.wasm

install-layouts:
	@mkdir -p $(LAYOUT_DIR)
	cp layout.kdl $(LAYOUT_DIR)/zjbar.kdl
	cp layout.swap.kdl $(LAYOUT_DIR)/zjbar.swap.kdl

install-hooks:
	scripts/install-hooks.sh

uninstall-hooks:
	scripts/install-hooks.sh --uninstall

install: build install-layouts
	@echo "Installed plugin and layouts."

uninstall:
	rm -f $(PLUGIN_DIR)/zjbar.wasm
	rm -f $(LAYOUT_DIR)/zjbar.kdl $(LAYOUT_DIR)/zjbar.swap.kdl
	-scripts/install-hooks.sh --uninstall 2>/dev/null
	@echo "Uninstalled."

clean:
	cargo clean

release: build
	@if [ -z "$(TAG)" ]; then \
		echo "Error: HEAD has no tag. Tag first with: git tag vX.Y.Z"; \
		exit 1; \
	fi
	gh release create $(TAG) $(WASM) --title "$(TAG)" --generate-notes
