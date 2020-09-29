#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
SCRIPT_NAME="$(basename "${BASH_SOURCE[0]}")"
SCRIPT="$SCRIPT_DIR/$SCRIPT_NAME"
WORKSPACE_DIR="$( dirname "$SCRIPT_DIR" )"
INSTANCE="$( basename "$WORKSPACE_DIR" )"
SCRIPT_SHORT_NAME="${SCRIPT_NAME%.*}"
LIBRARY_PATH_BASE=$(realpath $SCRIPT_DIR/../)

# echo "SCRIPT_DIR: $SCRIPT_DIR"
# echo "LIBRARY_PATH_BASE: $LIBRARY_PATH_BASE"

LD_DEBUG=
while true
do
    if [[ $1 == -ld_debug* ]];  then
        echo "Option: $1"
        if [[ $1 = -ld_debug=* ]];  then
            LD_DEBUG=${1##*-ld_debug=}
        else
            LD_DEBUG=libs
        fi
        echo "LD_DEBUG: $LD_DEBUG"
        shift
    elif [[ $1 == -trace* ]];  then
        echo "Option: $1"
        WISK_TRACE=`pwd`/wisktrace.log
        echo "WISK_TRACE: $WISK_TRACE"
        shift
    elif [[ $1 == -wsroot=* ]];  then
        echo "Option: $1"
        WISK_WSROOT=${1##*-wsroot=}
        echo "WISK_WSROOT: $WISK_WSROOT"
        shift
    else
        break
    fi
done

echo "Args: $*"

RUST_BACKTRACE=1
# rm -f `pwd`/wisktrack.pipe
# mknod `pwd`/wisktrack.pipe p
# WISK_TRACK=`pwd`/wisktrack.pipe
WISK_TRACK=`pwd`/wisktrack.file
LD_PRELOAD="$LIBRARY_PATH_BASE/\${LIB}/libwisktrack.so"
echo "LD_PRELOAD: $LD_PRELOAD"
echo "WISK_TRACK: $WISK_TRACK"

rm -f $WISK_TRACE
rm -rf $WISK_TRACK
echo "Starting....."
env -i LD_DEBUG="$LD_DEBUG" RUST_BACKTRACE="$RUST_BACKTRACE" TERM="$TERM" HOME="$HOME" LD_PRELOAD="$LD_PRELOAD" PATH="$PATH" USER="$USER" WISK_TRACE="$WISK_TRACE" WISK_TRACK="$WISK_TRACK" "$@"

