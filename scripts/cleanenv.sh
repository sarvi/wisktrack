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

DOSTRACE=
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
    elif [[ $1 == -strace ]];  then
        echo "Option: $1"
        DOSTRACE=true
        STRACE="strace -E LD_PRELOAD=$LIBRARY_PATH_BASE/\${LIB}/libwisktrack.so -E WISK_TRACK=/tmp/track.file -ff -q -o trace.log"
        echo "STRACE=$STRACE"
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
STRACEDIR="strace/"
echo "LD_PRELOAD: $LD_PRELOAD"
echo "WISK_TRACK: $WISK_TRACK"

rm -rf $STRACEDIR ; mkdir $STRACEDIR
rm -f $WISK_TRACE
rm -f $WISK_TRACE
rm -rf $WISK_TRACK
echo "Starting....."

if [[ -z $DOSTRACE ]]; then
env -i LD_DEBUG="$LD_DEBUG" RUST_BACKTRACE="$RUST_BACKTRACE" TERM="$TERM" HOME="$HOME" LD_PRELOAD="$LD_PRELOAD" PATH="$PATH" USER="$USER" WISK_TRACE="$WISK_TRACE" WISK_TRACK="$WISK_TRACK" "$@"
else
env -i strace -E LD_PRELOAD=$LIBRARY_PATH_BASE/\${LIB}/libwisktrack.so -ff -v -q -o $STRACEDIR/strace.log -E LD_DEBUG="$LD_DEBUG" -E RUST_BACKTRACE="$RUST_BACKTRACE" -E TERM="$TERM" -E HOME="$HOME" -E PATH="$PATH" -E USER="$USER" -E WISK_TRACE="$WISK_TRACE" -E WISK_TRACK="$WISK_TRACK" "$@"
fi
