# wisktrack
Filesystem Tracker
===================

This is a library that monitors and tracks all filesystem accesses for a program and all subprograms it invokes.

DESCRIPTION
-----------

The core functionality resides in libwisktrack.so that is built.
scripts/cleanenv.sh is a simple shell script that is used to testing and evaluation purpose is to simply
setup the environment and environment variables and invoke the program that is to be tracked.

The library writes data by default into wisktrack.file. But can be customized to write data to a different file
name of to separate files, one process invoked.


MAILINGLIST
-----------

