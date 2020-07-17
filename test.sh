#!/bin/bash

preload () {
    local library
    library=$1
    shift
    export RUST_BACKTRACE=full
    if [ "$(uname)" = "Darwin" ]; then
        DYLD_INSERT_LIBRARIES=target/debug/"$library".dylib "$@"
    else
        LD_PRELOAD=target/debug/"$library".so "$@"
    fi
}

set -ex
set -o pipefail
# export RUST_BACKTRACE=1
preload libwisktrack ls -l /dev/stdin | grep readlink

ln -s dummy test-panic
preload libwisktrack ls -l test-panic
rm test-panic

touch dummy
ln dummy linkdummy
preload libwisktrack ls -l linkdummy
rm linkdummy dummy


touch test.x
preload libwisktrack chmod a+rwx test.x
rm test.x