
ROOT?=/usr

SRCS:=src/lib.rs src/tracker.rs

target/debug/libwisktrack.so:  $(SRCS)
	cargo build
	cargo test

target/i686-unknown-linux-gnu/debug/libwisktrack.so:  $(SRCS)
	cargo build --target=i686-unknown-linux-gnu

$(ROOT)/lib/libwisktrack.so: target/debug/libwisktrack.so
	mkdir -p $(ROOT)/lib/
	install -D -m a+rwx target/debug/libwisktrack.so $(ROOT)/lib/

$(ROOT)/lib32/libwisktrack.so: target/i686-unknown-linux-gnu/debug/libwisktrack.so
	mkdir -p $(ROOT)/lib32/
	install -D -m a+rwx target/i686-unknown-linux-gnu/debug/libwisktrack.so $(ROOT)/lib32/

.PHONY: install
install: $(ROOT)/lib32/libwisktrack.so $(ROOT)/lib/libwisktrack.so

.PHONY: clean
clean:
	cargo clean
