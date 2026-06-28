.PHONY: build install run clean

build:
	cargo build --release

install: build
	mkdir -p ~/.local/bin
	install -m 755 ../target/release/cce-system-settings ~/.local/bin/cce-system-settings

run:
	cargo run

clean:
	cargo clean
