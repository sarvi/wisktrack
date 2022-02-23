#!/bin/bash
echo "execpre.sh:"
echo "WISK_EXECPREFIX:=$WISK_EXECPREFIX"
echo "PATH:=$PATH"
echo "PreExec: $0 $@"
while true
do
    if [[ $1 == --- ]];  then
        shift
        break
    else
        shift
    fi
done
echo "Exec: $*"
exec $*
