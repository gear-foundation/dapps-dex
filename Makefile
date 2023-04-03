.PHONY: all build fmt init lint pre-commit test deps

all: init build test

build:
	@echo ⚙️ Building a release...
	@cargo +nightly b -r
	@ls -l target/wasm32-unknown-unknown/release/*.wasm

fmt:
	@echo ⚙️ Checking a format...
	@cargo fmt --all --check

init:
	@echo ⚙️ Installing a toolchain \& a target...
	@rustup toolchain install nightly-2023-03-14 --component llvm-tools-preview
	@rustup target add wasm32-unknown-unknown --toolchain nightly-2023-03-14
	@rm -rf ~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu
	@ln -s ~/.rustup/toolchains/nightly-2023-03-14-x86_64-unknown-linux-gnu ~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu

lint:
	@echo ⚙️ Running the linter...
	@cargo +nightly clippy -- -D warnings
	@cargo +nightly clippy --all-targets -Fbinary-vendor -- -D warnings

pre-commit: fmt lint test

deps:
	@mkdir -p target
	@echo ⚙️ Downloading dependencies...
	@path=target/ft-main.wasm;\
	if [ ! -f $$path ]; then\
	    curl -L\
	        https://github.com/gear-dapps/sharded-fungible-token/releases/download/0.1.5/ft_main-0.1.5.opt.wasm\
	        -o $$path;\
	fi
	@path=target/ft-logic.wasm;\
	if [ ! -f $$path ]; then\
	    curl -L\
	        https://github.com/gear-dapps/sharded-fungible-token/releases/download/0.1.5/ft_logic-0.1.5.opt.wasm\
	        -o $$path;\
	fi
	@path=target/ft-storage.wasm;\
	if [ ! -f $$path ]; then\
	    curl -L\
	        https://github.com/gear-dapps/sharded-fungible-token/releases/download/0.1.5/ft_storage-0.1.5.opt.wasm\
	        -o $$path;\
	fi

test: deps
	@echo ⚙️ Running tests...
	@cargo +nightly t -Fbinary-vendor

full-test: deps
	@echo ⚙️ Running tests...
	@cargo +nightly t -Fbinary-vendor -- --include-ignored
