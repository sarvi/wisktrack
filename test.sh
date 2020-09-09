#!/bin/bash

preload () {
    local library
    library=$1
    shift
    if [ "$(uname)" = "Darwin" ]; then
        WISK_TRACE=/tmp/wisk_trace.log DYLD_INSERT_LIBRARIES=target/debug/"$library".dylib "$@"
    else
        WISK_TRACE=/tmp/wisk_trace.log LD_PRELOAD=target/debug/"$library".so "$@"
        # LD_PRELOAD=target/debug/"$library".so "$@"
    fi
}

set -ex
set -o pipefail

# cargo clean
# cargo update
cargo build
cc -Werror -o tests/testprog tests/test.c || exit "Testprog Compile Error"
rm -f /tmp/wisk_trace.log
touch /tmp/wisk_testfile
ln -sf /tmp/wisk_testfile /tmp/wisk_testlink
printf "\n\nRUST LD_PRELOAD"

preload libwisktrack ./tests/testprog creat-cw || exit
preload libwisktrack ./tests/testprog creat-r || exit

preload libwisktrack ./tests/testprog openat-cw || exit
preload libwisktrack ./tests/testprog openat-r || exit

test -f /tmp/wisk_trace.log && cat /tmp/wisk_trace.log
