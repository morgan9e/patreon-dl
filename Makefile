SRC_DIR := src
TARGET_DIR := target

all: build run

build:
	cargo build --release

run:
	cargo run

clean:
	cargo clean

test:
	cargo test

install:
	cargo install --path .

.PHONY: all build run clean test install
