package exec

import (
	"config"
	"fmt"
	"reflect"
	"testing"
)

func TestRunCmd(t *testing.T) {
	conf := config.Config{
		BaseDir:      "/ws/sarvi-sjc/wiskcache/exec",
		WiskTrackLib: "/ws/sarvi-sjc/wisktrack/${LIB}/libwisktrack.so",
	}
	testCases := []struct {
		d   string
		cmd []string
	}{
		{
			d:   "relative dirs",
			cmd: []string{"/bin/bash", "-c", "echo \"Hello World\" ; cat tests/file1.in > tests/file.out ; cat tests/file2.in >> tests/file.out ; cat tests/file2.in >> tests/file.out"},
		},
		{
			d:   "gcc compile",
			cmd: []string{"gcc", "-v", "-o", "tests/hello.o", "tests/hello.c"},
		},
		{
			d:   "move rename",
			cmd: []string{"/bin/bash", "tests/bashmove.sh"},
		},
	}
	for _, tc := range testCases {
		fmt.Println("\tSubTest: ", tc.d)
		exitcode, logfile, infiles, outfiles, _, _ := RunCmd(conf, "asdasdasd", tc.cmd)
		fmt.Println("Exec Failed: ", exitcode, logfile, infiles, outfiles, reflect.DeepEqual(infiles, []string{}))
		// if exitcode != 0 || !reflect.DeepEqual(infiles, []string{}) || !reflect.DeepEqual(outfiles, []string{}) {
		if exitcode != 0 { // || !reflect.DeepEqual(infiles, []string{}) || !reflect.DeepEqual(outfiles, []string{}) {
			fmt.Println("Exec Failed: ", exitcode, tc.cmd, infiles, outfiles)
			t.Fail()
		}

	}
}
