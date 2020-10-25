#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
SCRIPT_NAME="$(basename "${BASH_SOURCE[0]}")"
SCRIPT="$SCRIPT_DIR/$SCRIPT_NAME"
WORKSPACE_DIR="$( pwd -P )"
SCRIPT_SHORT_NAME="${SCRIPT_NAME%.*}"
LIBRARY_PATH_BASE=$(realpath $SCRIPT_DIR/../)

# echo "SCRIPT_DIR: $SCRIPT_DIR"
# echo "LIBRARY_PATH_BASE: $LIBRARY_PATH_BASE"

OSTYPE=
WISK_WSROOT="$WORKSPACE_DIR"
DODEBUG=false
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
    elif [[ $1 == -config=* ]];  then
        echo "Option: $1"
        WISK_CONFIG=${1##*-config=}
        echo "WISK_CONFIG: $WISK_CONFIG"
        shift
    elif [[ $1 == -strace ]];  then
        echo "Option: $1"
        DOSTRACE=true
        shift
    elif [[ $1 == -debug ]];  then
        echo "Option: $1"
        DODEBUG=true
        set -x
        shift
    elif [[ $1 == -os=* ]];  then
        echo "Option: $1"
        OSTYPE=${1##*-os=}
        echo "OSTYPE: $OSTYPE"
        shift
    else
        break
    fi
done

echo "Args: $*"

RUST_BACKTRACE=1
# WISK_TRACK=
# rm -f `pwd`/wisktrack.pipe
# mknod `pwd`/wisktrack.pipe p
# WISK_TRACK=`pwd`/wisktrack.pipe
WISK_TRACK=`pwd`/wisktrack.file
# WISK_TRACK=`pwd`/wisktrack
# WISK_TRACK=wisktrack.file
LD_PRELOAD="$LIBRARY_PATH_BASE/\${LIB}/libwisktrack.so"
# LD_PRELOAD="$LIBRARY_PATH_BASE/lib64/libwisktrack.so"
STRACEDIR="strace/"

echo "WISK_WSROOT: $WISK_WSROOT"
echo "LD_PRELOAD: $LD_PRELOAD"
echo "WISK_TRACK: $WISK_TRACK"
echo "PATH: $PATH"

rm -rf $STRACEDIR ; mkdir $STRACEDIR
rm -f $WISK_TRACE
rm -rf $WISK_TRACK

if [[ ! -f "$WISK_WSROOT/wisk/config/wisktrack.ini" ]]; then
   echo "Wisk Track Config not found at $WISK_WSROOT/wisk/config/wisktrack.ini"
   mkdir -p "$WISK_WSROOT/wisk/config"
   echo  "Installing $SCRIPT_DIR/../config/wisktrack.ini.$OSTYPE --> $WISK_WSROOT/wisk/config/wisktrack.ini"
   cp "$SCRIPT_DIR/../config/wisktrack.ini.$OSTYPE" "$WISK_WSROOT/wisk/config/wisktrack.ini"
   exit 1
fi
if [[ "$SCRIPT_DIR/../config/wisktrack.ini.$OSTYPE" -nt "$WISK_WSROOT/wisk/config/wisktrack.ini"]]; then
   echo  "Updating $SCRIPT_DIR/../config/wisktrack.ini.$OSTYPE --> $WISK_WSROOT/wisk/config/wisktrack.ini"
   cp "$SCRIPT_DIR/../config/wisktrack.ini.$OSTYPE" "$WISK_WSROOT/wisk/config/wisktrack.ini"
fi
echo "Starting....."

if [[ -z $DOSTRACE ]]; then
time env -i RUST_BACKTRACE="$RUST_BACKTRACE" TERM="$TERM" HOME="$HOME" LD_PRELOAD="$LD_PRELOAD" PATH="$PATH" USER="$USER" WISK_TRACE="$WISK_TRACE" WISK_TRACK="$WISK_TRACK" WISK_CONFIG="$WISK_CONFIG" WISK_WSROOT="$WISK_WSROOT" "$@"
else
time env -i strace -E LD_PRELOAD="$LD_PRELOAD" -ff -v -q -o $STRACEDIR/strace.log -E RUST_BACKTRACE="$RUST_BACKTRACE" -E TERM="$TERM" -E HOME="$HOME" -E PATH="$PATH" -E USER="$USER" -E WISK_TRACE="$WISK_TRACE" -E WISK_TRACK="$WISK_TRACK" -E WISK_CONFIG="$WISK_CONFIG" -E WISK_WSROOT="$WISK_WSROOT" "$@"
fi
