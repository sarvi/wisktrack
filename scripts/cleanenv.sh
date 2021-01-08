#!/bin/bash
set -x
tee2log() {
    local log_path

    [[ -n "${1:-}" ]] || { echo "Must supply log path" >&2; return 1; }
    log_path="$1"

    if [[ ! -f "$log_path" ]]; then
        mkdir -p "$( dirname "$log_path" )"
    fi

    # tee to log
    rm -f $WORKSPACE_DIR/logs/cleanenv.log
    echo "exec > >( tee -a $log_path ) 2>&1" >&2
    exec > >( tee -a "$log_path" ) 2>&1 || true
}
SCRIPT_DIR="$(cd "$(dirname $(realpath "${BASH_SOURCE[0]}"))" && pwd -P)"
SCRIPT_DIR=$(realpath $SCRIPT_DIR)
SCRIPT_NAME="$(basename "${BASH_SOURCE[0]}")"
SCRIPT="$SCRIPT_DIR/$SCRIPT_NAME"
SCRIPT=$(realpath $SCRIPT)
WORKSPACE_DIR="$( pwd -P )"
SCRIPT_SHORT_NAME="${SCRIPT_NAME%.*}"
LIBRARY_PATH_BASE=$(realpath $SCRIPT_DIR/../)

umask 000

mkdir -p $WORKSPACE_DIR/logs
tee2log $WORKSPACE_DIR/logs/cleanenv.log

# echo "SCRIPT_DIR: $SCRIPT_DIR"
# echo "LIBRARY_PATH_BASE: $LIBRARY_PATH_BASE"

OSTYPE=
WISK_WSROOT="$WORKSPACE_DIR"
DODEBUG=false
DOSTRACE=
LD_DEBUG=
DOSTAP=
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
    elif [[ $1 == -stap* ]];  then
        echo "Option: $1"
        if [[ $1 = -stap=* ]];  then
            DOSTAP=${1##*-stap=}
        else
            DOSTAP=$LIBRARY_PATH_BASE/stap/default.stp
        fi
        echo "DOSTAP: $DOSTAP"
        shift
        exec sudo stap -v $DOSTAP -d /usr/bin/bash -d `ls /lib64/libc-*` -d $LIBRARY_PATH_BASE/lib64/libwisktrack.so -d $LIBRARY_PATH_BASE/lib32/libwisktrack.so -o $WORKSPACE_DIR/logs/cleanenv.stap.log -c "$SCRIPT $*"
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
WISK_TRACK=$WISK_WSROOT/wisktrack.file
# WISK_TRACK=`pwd`/wisktrack.file
# WISK_TRACK=`pwd`/wisktrack
# WISK_TRACK=wisktrack.file
WISK_TRACK=
LD_PRELOAD="$LIBRARY_PATH_BASE/\${LIB}/libwisktrack.so"
STRACEDIR="strace/"

echo "WISK_WSROOT: $WISK_WSROOT"
echo "LD_PRELOAD: $LD_PRELOAD"
echo "WISK_TRACK: $WISK_TRACK"
echo "PATH: $PATH"

rm -rf $STRACEDIR ; mkdir $STRACEDIR
rm -f $WISK_TRACE
if [[ -z "$WISK_TRACK" ]]; then
    rm -rf $WISK_WSROOT/wisktrack.file
else
    rm -rf $WISK_TRACK
fi

if [[ ! -f "$WISK_WSROOT/wisk/config/wisktrack.ini" ]]; then
   echo "Wisk Track Config not found at $WISK_WSROOT/wisk/config/wisktrack.ini"
   mkdir -p "$WISK_WSROOT/wisk/config"
   echo  "Installing $SCRIPT_DIR/../config/wisktrack.ini.$OSTYPE --> $WISK_WSROOT/wisk/config/wisktrack.ini"
   cp "$SCRIPT_DIR/../config/wisktrack.ini.$OSTYPE" "$WISK_WSROOT/wisk/config/wisktrack.ini"
   exit 1
fi
if [[ "$SCRIPT_DIR/../config/wisktrack.ini.$OSTYPE" -nt "$WISK_WSROOT/wisk/config/wisktrack.ini" ]]; then
   echo  "Updating $SCRIPT_DIR/../config/wisktrack.ini.$OSTYPE --> $WISK_WSROOT/wisk/config/wisktrack.ini"
   cp "$SCRIPT_DIR/../config/wisktrack.ini.$OSTYPE" "$WISK_WSROOT/wisk/config/wisktrack.ini"
fi
echo "Starting....."

if [[ ! -z $DOSTRACE ]]; then
# time env -i strace -E LD_PRELOAD="/nobackup/sarvi/iosxr/platforms/common/thinxr/build/obj-x86-linux/libcpio_preload.so:$LD_PRELOAD" -ff -v -s 1024 -q -o $STRACEDIR/strace.log -E RUST_BACKTRACE="$RUST_BACKTRACE" -E TERM="$TERM" -E HOME="$HOME" -E PATH="$PATH" -E USER="$USER" -E WISK_TRACE="$WISK_TRACE" -E WISK_TRACK="$WISK_TRACK" -E WISK_CONFIG="$WISK_CONFIG" -E WISK_WSROOT="$WISK_WSROOT" "$@"
time env -i strace -E LD_PRELOAD="$LD_PRELOAD" -ff -v -s 1024 -q -o $STRACEDIR/strace.log -E RUST_BACKTRACE="$RUST_BACKTRACE" -E TERM="$TERM" -E HOME="$HOME" -E PATH="$PATH" -E USER="$USER" -E WISK_TRACE="$WISK_TRACE" -E WISK_TRACK="$WISK_TRACK" -E WISK_CONFIG="$WISK_CONFIG" -E WISK_WSROOT="$WISK_WSROOT" "$@"
else
time env -i RUST_BACKTRACE="$RUST_BACKTRACE" TERM="$TERM" HOME="$HOME" LD_PRELOAD="$LD_PRELOAD" PATH="$PATH" USER="$USER" WISK_TRACE="$WISK_TRACE" WISK_TRACK="$WISK_TRACK" WISK_CONFIG="$WISK_CONFIG" WISK_WSROOT="$WISK_WSROOT" "$@"
fi
echo "Logfile: $WORKSPACE_DIR/logs/cleanenv.log"
