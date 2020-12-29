#define _GNU_SOURCE

#include <stdio.h>
#include <stdlib.h>
#include <stdarg.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>  
#include <string.h>

int testreadlink(const char *link)
{
    char buf[500];
    readlink(link, buf, 500);
}

int testvprintf(const char *format, ...)
{
    va_list argp;

    va_start(argp, format);
    vprintf(format, argp);
    va_end(argp);

}

int testprintf(const char *str, int i, float f, char *s)
{
    printf(str, i, f, s);
}


int main(int argc, char *argv[])
{
    printf("testprog running......\n");
    if (argc != 2) {
        puts("Command: takes one argument. Which system call to run");
        puts("Options: readlink, vprintf, printf, open, fopen, creat");
        exit(-1);
    }
    if (strcmp(argv[1], "readlink") == 0) {
        testreadlink("/tmp/wisk_testlink");
    } else if (strcmp(argv[1], "vprintf") == 0) {
        testvprintf("Hello World! from vprintf: %d %f %s \n", 100, 1.23456, "something");
    } else if (strcmp(argv[1], "printf") == 0) {
        testprintf("Hello World! from printf: %d %f %s \n", 100, 1.23456, "something");
    } else if (strcmp(argv[1], "close-800") == 0) {
        close(800);
    } else if (strcmp(argv[1], "creat-cw") == 0) {
        close(open("/tmp/created.file", O_CREAT|O_WRONLY, 0));
    } else if (strcmp(argv[1], "creat-r") == 0) {
        close(open("/tmp/created.file", O_RDONLY, 0));
    } else if (strcmp(argv[1], "open-cw") == 0) {
        close(open("/tmp/open.file", O_CREAT|O_WRONLY, 0));
    } else if (strcmp(argv[1], "open-r") == 0) {
        close(open("/tmp/open.file", O_RDONLY, 0));
    } else if (strcmp(argv[1], "open64-cw") == 0) {
        close(open64("/tmp/open64.file", O_CREAT|O_WRONLY, 0));
    } else if (strcmp(argv[1], "open64-r") == 0) {
        close(open64("/tmp/open64.file", O_RDONLY, 0));
    } else if (strcmp(argv[1], "openat-cw") == 0) {
        close(openat(AT_FDCWD, "/tmp/opennat.file", O_CREAT|O_WRONLY, 0));
    } else if (strcmp(argv[1], "openat-r") == 0) {
        close(openat(AT_FDCWD, "/tmp/openat.file", O_RDONLY, 0));
    } else if (strcmp(argv[1], "fopen-cw") == 0) {
        fclose(fopen("/tmp/fopen.file", "w"));
    } else if (strcmp(argv[1], "fopen-r") == 0) {
        fclose(fopen("/tmp/fopen.file", "r"));
    } else if (strcmp(argv[1], "fopen64-cw") == 0) {
        fclose(fopen64("/tmp/fopen64.file", "w"));
    } else if (strcmp(argv[1], "fopen64-r") == 0) {
        fclose(fopen64("/tmp/fopen64.file", "r"));
    } else if (strcmp(argv[1], "execv") == 0) {
        char *eargv[] = {"ls", "-l", "/usr/bin/ls", NULL};
        execv("/bin/ls", eargv);
    } else if (strcmp(argv[1], "execvp") == 0) {
        char *eargv[] = {"ls", "-l", "/usr/bin/ls", NULL};
        execvp("ls", eargv);
    } else if (strcmp(argv[1], "execvp_pwd") == 0) {
        char *eargv[] = {"/bin/pwd", NULL};
        execvp("/bin/pwd", eargv);
    } else if (strcmp(argv[1], "execvpe") == 0) {
        char *eargv[] = {"ls", "-l", "/usr/bin/ls", NULL};
        char *env[] = {"PATH=/nothing:", NULL};
        execvpe("ls", eargv, env);
    } else if (strcmp(argv[1], "execve") == 0) {
        char *eargv[] = {"ls", "-l", "/usr/bin/ls", NULL};
        char *env[] = {"PATH=/usr/bin:", NULL};
        execve("/bin/ls", eargv, env);
    } else if (strcmp(argv[1], "execl") == 0) {
        execl("/bin/ls", "ls", "-l", "/usr/bin/ls", NULL);
    } else if (strcmp(argv[1], "execlp") == 0) {
        execlp("ls", "ls", "-l", "/usr/bin/ls", NULL);
    } else if (strcmp(argv[1], "execlpscript") == 0) {
        execlp("wit", "wit", "--verson", NULL);
    } else if (strcmp(argv[1], "execle") == 0) {
        char *env[] = {"PATH=/nothing:", NULL};
        execle("/bin/ls", "ls", "-l", "/usr/bin/ls", NULL, env);
    } else if (strcmp(argv[1], "segfault") == 0) {
        int *intptr=NULL;
        *(intptr) = 0xffff;
    } else {
        puts("Command: takes one argument. Which system call to run");
        puts("Options: readlink, vprintf, printf, open, fopen, creat");
        exit(-1);
    }
    return 0;
}
