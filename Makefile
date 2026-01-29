.PHONY: all build build-web build-bundler build-nodejs test clean

all: build

# Build for web (browser with ES modules)
build: build-web

build-web:
	wasm-pack build --target web --out-dir pkg --release

# Build for bundlers (webpack, vite, etc.)
build-bundler:
	wasm-pack build --target bundler --out-dir pkg/bundler --release

# Build for Node.js
build-nodejs:
	wasm-pack build --target nodejs --out-dir pkg/nodejs --release

# Build all targets
build-all: build-web build-bundler build-nodejs

# Run tests in headless browser
test:
	wasm-pack test --headless --firefox

test-chrome:
	wasm-pack test --headless --chrome

# Clean build artifacts
clean:
	rm -rf pkg target

# Development build (faster, with debug info)
dev:
	wasm-pack build --target web --out-dir pkg --dev

# Install wasm-pack if not present
setup:
	@command -v wasm-pack >/dev/null 2>&1 || cargo install wasm-pack
	@command -v wasm-bindgen >/dev/null 2>&1 || cargo install wasm-bindgen-cli

# Check that Rust toolchain has wasm target
check-toolchain:
	@rustup target list --installed | grep -q wasm32-unknown-unknown || rustup target add wasm32-unknown-unknown
