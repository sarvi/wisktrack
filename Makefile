
ROOT?=/usr

SRCS:=src/lib.rs src/tracker.rs src/utils.rs

tests/testprog64: tests/test.c
	cc -Werror -o tests/testprog64 tests/test.c

tests/testprog32: tests/test.c
	cc -Werror -m32 -o tests/testprog32 tests/test.c

target/debug/libwisktrack.so:  $(SRCS)
	cargo build
	cargo test || true

target/debug/libwisktrack.so:  $(SRCS)
	cargo build
	cargo test || true

target/i686-unknown-linux-gnu/debug/libwisktrack.so:  $(SRCS)
	cargo build --target=i686-unknown-linux-gnu

$(ROOT)/lib/libwisktrack.so: target/debug/libwisktrack.so
	mkdir -p $(ROOT)/lib64/
	install -D -m a+rwx target/debug/libwisktrack.so $(ROOT)/lib64/

$(ROOT)/lib32/libwisktrack.so: target/i686-unknown-linux-gnu/debug/libwisktrack.so
	mkdir -p $(ROOT)/lib32/
	install -D -m a+rwx target/i686-unknown-linux-gnu/debug/libwisktrack.so $(ROOT)/lib32/

$(ROOT)/bin/cleanenv.sh: scripts/cleanenv.sh
	mkdir -p $(ROOT)/bin/
	install -D -m a+rwx scripts/cleanenv.sh $(ROOT)/bin/

.PHONY: tests
tests: tests/testprog64 tests/testprog32

.PHONY: all
all: target/i686-unknown-linux-gnu/debug/libwisktrack.so target/debug/libwisktrack.so tests

.PHONY: install
install: $(ROOT)/lib32/libwisktrack.so $(ROOT)/lib/libwisktrack.so $(ROOT)/bin/cleanenv.sh

.PHONY: clean
clean:
	cargo clean
