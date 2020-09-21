#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
SCRIPT_NAME="$(basename "${BASH_SOURCE[0]}")"
SCRIPT="$SCRIPT_DIR/$SCRIPT_NAME"
WORKSPACE_DIR="$( dirname "$SCRIPT_DIR" )"
INSTANCE="$( basename "$WORKSPACE_DIR" )"
SCRIPT_SHORT_NAME="${SCRIPT_NAME%.*}"

echo "SCRIPT_DIR: $SCRIPT_DIR"

if [[ "$SCRIPT_DIR" == */bin ]]
then
    echo "Installed:"
else
    echo "Workspace:"
fi
RUST_BACKTRACE=1
WISK_TRACE=`pwd`/wisktrace.log
# rm -f `pwd`/wisktrack.pipe
# mknod `pwd`/wisktrack.pipe p
# WISK_TRACKFILE=`pwd`/wisktrack.pipe
WISK_TRACKFILE=`pwd`/wisktrack.file
LD_PRELOAD=libwisktrack.so
LD_LIBRARY_PATH=$SCRIPT_DIR/../lib32:$SCRIPT_DIR/../lib64
echo "PATH: $PATH"
echo "LD_PRELOAD: $LD_PRELOAD"
echo "LD_LIBRARY_PATH: $LD_LIBRARY_PATH"
echo "WISK_TRACE: $WISK_TRACE"
echo "WISK_TRACKFILE: $WISK_TRACKFILE"

rm -f $WISK_TRACE
rm -f $WISK_TRACKFILE
env -i RUST_BACKTRACE="$RUST_BACKTRACE" HOME="$HOME" LD_PRELOAD="$LD_PRELOAD" LD_LIBRARY_PATH="$LD_LIBRARY_PATH" PATH="$PATH" USER="$USER" WISK_TRACE="$WISK_TRACE" WISK_TRACKFILE="$WISK_TRACKFILE" "$@"

