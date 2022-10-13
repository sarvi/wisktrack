package whash

import (
	"config"
	"fmt"
	"reflect"
	"testing"
)

type TestData struct {
	conf config.Config
	env  map[string]string
	cmd  []string
}

type TestDataPair struct {
	d string
	l TestData
	r TestData
}

func TestCmdNormalize(t *testing.T) {
	conf := config.Config{
		BaseDir: "/a/b",
	}
	testCases := []struct {
		d       string
		basedir string
		cmd     []string
		ncmd    []string
	}{
		{
			d:       "relative dirs",
			basedir: "/a/b",
			cmd:     []string{"g++", "-o", "file.o", "file.c"},
			ncmd:    []string{"g++", "-o", "file.o", "file.c"},
		},
		{
			d:       "absolute and relative dirs",
			basedir: "/a/b",
			cmd:     []string{"g++", "-o", "file.o", "/a/b/file.c"},
			ncmd:    []string{"g++", "-o", "file.o", "./file.c"},
		},
	}
	for _, tc := range testCases {
		fmt.Println("\tSubTest: ", tc.d)
		conf.BaseDir = tc.basedir
		x := cmdnormalize(conf, tc.cmd)
		if !reflect.DeepEqual(x, tc.ncmd) {
			fmt.Println("Match Failed: ", tc.cmd, x, tc.ncmd)
			t.Fail()
		}

	}
}

func TestEnvNormalize(t *testing.T) {
	conf := config.Config{
		BaseDir: "/a/b",
	}
	testCases := []struct {
		d       string
		basedir string
		env     map[string]string
		nenv    map[string]string
	}{
		{
			d:       "absolute and relative dirs",
			basedir: "/a/b",
			env:     map[string]string{"CWD": "/a/b/c/d"},
			nenv:    map[string]string{"CWD": "./c/d"},
		},
		{
			d:       "relative and relative dirs",
			basedir: "/a/b",
			env:     map[string]string{"CWD": "d"},
			nenv:    map[string]string{"CWD": "d"},
		},
	}
	for _, tc := range testCases {
		fmt.Println("\tSubTest: ", tc.d)
		conf.BaseDir = tc.basedir
		x := envnormalize(conf, tc.env)
		if !reflect.DeepEqual(x, tc.nenv) {
			fmt.Println("Match Failed: ", tc.env, x, tc.nenv)
			t.Fail()
		}

	}
}

func TestHashMustMatch(t *testing.T) {
	testCases := []TestDataPair{
		{
			d: "Same all",
			l: TestData{
				conf: config.Config{
					BaseDir: "/a/b",
					Envars:  []string{"CWD"},
					Tools:   []config.Tool{},
					ToolIdx: -1,
				},
				env: map[string]string{"CWD": "/a/b/c"},
				cmd: []string{"g++", "-o", "file.o", "file.c"},
			},
			r: TestData{
				conf: config.Config{
					BaseDir: "/a/b",
					Envars:  []string{"CWD"},
					Tools:   []config.Tool{},
					ToolIdx: -1,
				},
				env: map[string]string{"CWD": "/a/b/c"},
				cmd: []string{"g++", "-o", "file.o", "file.c"},
			},
		},
		{
			d: "Same command and env, absolute vs relative path args",
			l: TestData{
				conf: config.Config{
					BaseDir: "/a/b",
					Envars:  []string{"CWD"},
					Tools:   []config.Tool{},
					ToolIdx: -1,
				},
				env: map[string]string{"CWD": "/a/b/c"},
				cmd: []string{"g++", "-o", "file.o", "/a/b/c/file.c"},
			},
			r: TestData{
				conf: config.Config{
					BaseDir: "/a/b",
					Envars:  []string{"CWD"},
					Tools:   []config.Tool{},
					ToolIdx: -1,
				},
				env: map[string]string{"CWD": "/a/b/c"},
				cmd: []string{"g++", "-o", "file.o", "./c/file.c"},
			},
		},
	}
	for _, tc := range testCases {
		fmt.Println("\tSubTest: ", tc.d)
		h1, e1 := CommandHash(tc.l.conf, tc.l.env, tc.l.cmd)
		h2, e2 := CommandHash(tc.r.conf, tc.r.env, tc.r.cmd)
		// fmt.Println(h1, h2)
		if h1 != h2 || e1 != nil || e2 != nil {
			t.Fail()
		}
	}
}

func TestHashMustNotMatch(t *testing.T) {
	testCases := []TestDataPair{
		{
			d: "Same command, different CWD",
			l: TestData{
				conf: config.Config{
					BaseDir: "/a/b",
					Envars:  []string{"CWD"},
					Tools:   []config.Tool{},
					ToolIdx: -1,
				},
				env: map[string]string{"CWD": "/a/b/c"},
				cmd: []string{"g++", "-o", "file.o", "file.c"},
			},
			r: TestData{
				conf: config.Config{
					BaseDir: "/a/b",
					Envars:  []string{"CWD"},
					Tools:   []config.Tool{},
					ToolIdx: -1,
				},
				env: map[string]string{"CWD": "/a/b/d"},
				cmd: []string{"g++", "-o", "file.o", "file.c"},
			},
		},
	}
	for _, tc := range testCases {
		fmt.Println("\tSubTest: ", tc.d)
		h1, e1 := CommandHash(tc.l.conf, tc.l.env, tc.l.cmd)
		h2, e2 := CommandHash(tc.r.conf, tc.r.env, tc.r.cmd)
		// fmt.Println(h1, h2)
		if h1 == h2 || e1 != nil || e2 != nil {
			t.Fail()
		}
	}
}
