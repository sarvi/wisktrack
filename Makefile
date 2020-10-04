
ROOT?=/usr

SRCS:=src/lib.rs src/tracker.rs src/utils.rs

.PHONY: basics
basics:
	echo "Basics..."
	ln -sf target/i686-unknown-linux-gnu/debug lib
	ln -sf target/i686-unknown-linux-gnu/debug lib32
	ln -sf target/debug lib64

tests/testprog64: tests/test.c
	echo "tests/testprog64"
	cc -Werror -o tests/testprog64 tests/test.c

tests/testprog32: tests/test.c
	echo "tests/testprog32"
	cc -Werror -m32 -o tests/testprog32 tests/test.c

target/debug/libwisktrack.so:  $(SRCS)
	echo "cargo build 64bit"
	cargo build

.PHONY: cargo-tests
cargo-tests:
	echo "cargo test 64bit"
	cargo test || true

target/i686-unknown-linux-gnu/debug/libwisktrack.so:  $(SRCS)
	echo "cargo build 32bit"
	cargo build --target=i686-unknown-linux-gnu

$(ROOT)/lib/libwisktrack.so: target/debug/libwisktrack.so
	echo "install 64bit"
	mkdir -p $(ROOT)/lib64/
	install -D -m a+rwx target/debug/libwisktrack.so $(ROOT)/lib64/

$(ROOT)/lib32/libwisktrack.so: target/i686-unknown-linux-gnu/debug/libwisktrack.so
	echo "install 32bit"
	mkdir -p $(ROOT)/lib32/
	install -D -m a+rwx target/i686-unknown-linux-gnu/debug/libwisktrack.so $(ROOT)/lib32/
	ln -sf lib32 $(ROOT)/lib

$(ROOT)/bin/cleanenv.sh: scripts/cleanenv.sh
	echo "install scripts"
	mkdir -p $(ROOT)/bin
	mkdir -p $(ROOT)/config
	install -D scripts/cleanenv.sh $(ROOT)/bin/
	install -D config/default.ini $(ROOT)/config/

.PHONY: tests
tests: tests/testprog64 tests/testprog32 basics | cargo-tests

.PHONY: all
all: target/i686-unknown-linux-gnu/debug/libwisktrack.so target/debug/libwisktrack.so basics | cargo-tests

.PHONY: install
install: $(ROOT)/lib32/libwisktrack.so $(ROOT)/lib/libwisktrack.so $(ROOT)/bin/cleanenv.sh

.PHONY: clean
clean:
	cargo clean
