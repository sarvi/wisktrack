LD_LIBRARY_PATH=/usr/cisco/packages/oracle/current/lib:/usr/cisco/packages/oracle/current/lib:/usr/cisco/packages/oracle/current/lib:
UNFSD_DIR=/nfs/nova/tools/latest/bin
SSH_CONNECTION=10.24.6.214 64735 171.68.196.186 22
MODULES_RUN_QUARANTINE=LD_LIBRARY_PATH
LANG=en_US.UTF-8
HISTCONTROL=ignoredups
ORACLE_HOME=/usr/cisco/packages/oracle/current
HOSTNAME=sjc-ads-5990
AMD_ENTRYPOINT=vs/server/remoteExtensionHostProcess
OLDPWD=/users/sarvi/.vscode-server/bin/91899dcef7b8110878ea59626991a18c8a6a1b3e
COLORTERM=truecolor
SSH_AUTH_SOCK=/run/user/19375/vscode-ssh-auth-sock-683510985
S_COLORS=auto
BGLHOST=bgl-ads-2751
APPLICATION_INSIGHTS_NO_DIAGNOSTIC_CHANNEL=true
XDG_SESSION_ID=782
MODULES_CMD=/usr/share/Modules/libexec/modulecmd.tcl
USER=sarvi
ENV=/usr/share/Modules/init/profile.sh
PWD=/ws/sarvi-sjc/wisktrack
SSH_ASKPASS=/usr/libexec/openssh/gnome-ssh-askpass
HOME=/users/sarvi
VSCODE_GIT_ASKPASS_NODE=/ws/sarvi-sjc/.vscode-server/bin/91899dcef7b8110878ea59626991a18c8a6a1b3e/node
TERM_PROGRAM=vscode
SSH_CLIENT=10.24.6.214 64735 22
TERM_PROGRAM_VERSION=1.47.3
BASH_ENV=/usr/share/Modules/init/bash
XDG_DATA_DIRS=/users/sarvi/.local/share/flatpak/exports/share:/var/lib/flatpak/exports/share:/usr/local/share:/usr/share
ACME_DIFF_OPTS=-C 5 -p
VSCODE_IPC_HOOK_CLI=/tmp/vscode-ipc-f14c2782-0ab8-4db4-87c0-6b3c05ffd879.sock
MAIL=/var/spool/mail/sarvi
VSCODE_GIT_ASKPASS_MAIN=/ws/sarvi-sjc/.vscode-server/bin/91899dcef7b8110878ea59626991a18c8a6a1b3e/extensions/git/dist/askpass-main.js
TERM=xterm-256color
SHELL=/bin/bash
SHLVL=5
VSCODE_GIT_IPC_HANDLE=/run/user/19375/vscode-git-835507c14f.sock
MANPATH=/opt/quest/man::/opt/puppetlabs/puppet/share/man
TOOLSDIR=/nfs/nova/tools/latest
MODULEPATH=/etc/scl/modulefiles:/etc/scl/modulefiles:/usr/share/Modules/modulefiles:/etc/modulefiles:/usr/share/modulefiles
PIPE_LOGGING=true
LOGNAME=sarvi
DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/19375/bus
GIT_ASKPASS=/ws/sarvi-sjc/.vscode-server/bin/91899dcef7b8110878ea59626991a18c8a6a1b3e/extensions/git/dist/askpass.sh
XDG_RUNTIME_DIR=/run/user/19375
MODULEPATH_modshare=/usr/share/modulefiles:1:/etc/modulefiles:1:/usr/share/Modules/modulefiles:1
PATH=/users/sarvi/.cargo/bin:/auto/binos-tools/bin:/router/bin:/usr/cisco/bin:/usr/atria/bin:/usr/bin:/usr/local/bin:/usr/local/etc:/bin:/usr/X11R6/bin:/usr/sbin:/sbin:/usr/bin:/auto/nova-env/Lab/Labpatch/bin
MODULESHOME=/usr/share/Modules
HISTSIZE=1000
LESSOPEN=||/usr/bin/lesspipe.sh %s
VERBOSE_LOGGING=true
BASH_FUNC_module%%=() {  unset _mlre _mlIFS _mlshdbg;
 if [ "${MODULES_SILENT_SHELL_DEBUG:-0}" = '1' ]; then
 case "$-" in 
 *v*x*)
 set +vx;
 _mlshdbg='vx'
 ;;
 *v*)
 set +v;
 _mlshdbg='v'
 ;;
 *x*)
 set +x;
 _mlshdbg='x'
 ;;
 *)
 _mlshdbg=''
 ;;
 esac;
 fi;
 if [ -n "${IFS+x}" ]; then
 _mlIFS=$IFS;
 fi;
 IFS=' ';
 for _mlv in ${MODULES_RUN_QUARANTINE:-};
 do
 if [ "${_mlv}" = "${_mlv##*[!A-Za-z0-9_]}" -a "${_mlv}" = "${_mlv#[0-9]}" ]; then
 if [ -n "`eval 'echo ${'$_mlv'+x}'`" ]; then
 _mlre="${_mlre:-}${_mlv}_modquar='`eval 'echo ${'$_mlv'}'`' ";
 fi;
 _mlrv="MODULES_RUNENV_${_mlv}";
 _mlre="${_mlre:-}${_mlv}='`eval 'echo ${'$_mlrv':-}'`' ";
 fi;
 done;
 if [ -n "${_mlre:-}" ]; then
 eval `eval ${_mlre}/usr/bin/tclsh /usr/share/Modules/libexec/modulecmd.tcl bash '"$@"'`;
 else
 eval `/usr/bin/tclsh /usr/share/Modules/libexec/modulecmd.tcl bash "$@"`;
 fi;
 _mlstatus=$?;
 if [ -n "${_mlIFS+x}" ]; then
 IFS=$_mlIFS;
 else
 unset IFS;
 fi;
 if [ -n "${_mlshdbg:-}" ]; then
 set -$_mlshdbg;
 fi;
 unset _mlre _mlv _mlrv _mlIFS _mlshdbg;
 return $_mlstatus
}
BASH_FUNC_switchml%%=() {  typeset swfound=1;
 if [ "${MODULES_USE_COMPAT_VERSION:-0}" = '1' ]; then
 typeset swname='main';
 if [ -e /usr/share/Modules/libexec/modulecmd.tcl ]; then
 typeset swfound=0;
 unset MODULES_USE_COMPAT_VERSION;
 fi;
 else
 typeset swname='compatibility';
 if [ -e /usr/share/Modules/libexec/modulecmd-compat ]; then
 typeset swfound=0;
 MODULES_USE_COMPAT_VERSION=1;
 export MODULES_USE_COMPAT_VERSION;
 fi;
 fi;
 if [ $swfound -eq 0 ]; then
 echo "Switching to Modules $swname version";
 source /usr/share/Modules/init/bash;
 else
 echo "Cannot switch to Modules $swname version, command not found";
 return 1;
 fi
}
BASH_FUNC_scl%%=() {  if [ "$1" = "load" -o "$1" = "unload" ]; then
 eval "module $@";
 else
 /usr/bin/scl "$@";
 fi
}
_=/usr/bin/env
