package cache

import (
	"fmt"
	"testing"
	"sync"
	"os/exec"
	"strings"
	"os"
	"config"
	"path/filepath"
	"utils"
	"io/ioutil"
)

func TestIsUpper(t *testing.T) {
	fmt.Println("Test IsUpper Cache")
	Greet()
}

func TestIsLower(t *testing.T) {
	fmt.Println("Test IsLower Cache")
	//t.Fail()
}

func TestCopyOutInParallel(t *testing.T) {
	fmt.Println("Test CopyOutINparallel")
	cmd := exec.Command("fallocate", "-l", "1G", "/tmp/1gfile1")
	err := cmd.Run()
        if err != nil{
		fmt.Println("Failed to create files for testing")
		return
        }
	cmd = exec.Command("fallocate", "-l", "1G", "/tmp/1gfile2")
	err = cmd.Run()
        if err != nil{
		fmt.Println("Failed to create files for testing")
		return
        }
	var wg sync.WaitGroup
	outfile := []string{"/tmp/1gfile1", "/tmp/1gfile2"}
	for _, ofile := range outfile{
		target := ofile + ".dest"
		fmt.Printf("Copying %v to %v\n", ofile, target)
		wg.Add(1)
		go func(src string, tgt string){
			defer wg.Done()
			cpCmd := exec.Command("cp", src, tgt)
			cperr := cpCmd.Run()
			if cperr != nil{
	            		err = cperr
			}
		}(ofile, target)
	}
	//cmd = exec.Command("ps", "-ef", "\\|", "grep", "cp")
	cmd = exec.Command("ps", "-ef")
	output, _ := cmd.CombinedOutput()
	wg.Wait()
	os.Remove("/tmp/1gfile1")
	os.Remove("/tmp/1gfile2")
	os.Remove("/tmp/1gfile1.dest")
	os.Remove("/tmp/1gfile2.dest")
	if !strings.Contains(string(output), "cp /tmp/1gfile1 /tmp/1gfile1.dest") || !strings.Contains(string(output), "cp /tmp/1gfile2 /tmp/1gfile2.dest"){
		t.Fail()
	} 
}

func TestManifest(t *testing.T) {
	fmt.Println("Test Manifest")
	var config config.Config
	config.CacheBaseDir = "/tmp/cache_testTestFindManifest"
	cmdhash := "cmdhash001"
	manifestFile, _ := FindManifest(config, cmdhash)
	infiles := []string{"../test/hello.c", "../test/square.h", "../test/sum.h"}
	outfiles := []string{"../test/hello.o"}
	symlinks := [][2]string{}
	if filepath.Base(manifestFile) == "manifest.base" {
		cmd := exec.Command("touch", "../test/hello.o")
		cmd.Run()
		// test create a new manifest file
		manifestFile, _ = Create(config, "", infiles, outfiles, symlinks, manifestFile)
		if manifestFile != "/tmp/cache_testTestFindManifest/cmdhash001/manifest.21ba3c3e8d81bad2c995ba1b89b708e850f737914227001fb6d0fa2156cda1ea" {
			t.Fail()
                }
		// test CopyOut from cache
		if utils.Exists("../test/hello.o") {
			os.Remove("../test/hello.o")
		}
		CopyOut(config, manifestFile)
		if !utils.Exists("../test/hello.o") {
			t.Fail()
		}
		// test find cache
	        manifestFile, _ = FindManifest(config, cmdhash)
		if manifestFile != "/tmp/cache_testTestFindManifest/cmdhash001/manifest.21ba3c3e8d81bad2c995ba1b89b708e850f737914227001fb6d0fa2156cda1ea" {
			t.Fail()
		}
		// modify one inFile
		cmd = exec.Command("cp", "../test/sum.h", "../test/sum.h.bak")
		cmd.Run()
		ioutil.WriteFile("../test/sum.h", []byte("hello\n"), 0644)
	        manifestFile, _ = FindManifest(config, cmdhash)
		// should no manifestFile found in cache
		if utils.Exists(manifestFile) {
			fmt.Printf("Found %v which should not be there\n", manifestFile)
			t.Fail()
		}
		manifestFile, _ = Create(config, "", infiles, outfiles, symlinks, manifestFile)
		cmd = exec.Command("cp", "../test/sum.h.bak", "../test/sum.h")
		cmd.Run()
		os.Remove("../test/sum.h.bak")
		// after revert back sum.bak, should find manifest in the cache 
		manifestFile, _ = FindManifest(config, cmdhash)
		fmt.Println(manifestFile)
                if manifestFile != "/tmp/cache_testTestFindManifest/cmdhash001/manifest.21ba3c3e8d81bad2c995ba1b89b708e850f737914227001fb6d0fa2156cda1ea" {
                        t.Fail()
                }
        }else{
		t.Fail()
	}
	os.RemoveAll("/tmp/cache_testTestFindManifest")
}

func setUpTestFiles(src []string, tgt []string){
	for f_index, srcFile := range src{
		cmd := exec.Command("cp", srcFile, tgt[f_index])
		cmd.Run()
	}
}

func TestFindManifest(t *testing.T) {
	fmt.Println("Test FindManifest")
	var config config.Config
	config.CacheBaseDir = "/tmp/cache_testTestFindManifest"
	cmdhash := "cmdhash001"
	manifestFile, _ := FindManifest(config, cmdhash)
	infiles := []string{"../test/file1_to_test", "../test/file2_to_test", "../test/file3_to_test", "../test/file4_to_test", "../test/file5_to_test"}
	setUpTestFiles([]string{"../test/1line","../test/1line","../test/1line","../test/1line","../test/1line"}, infiles)
	outfiles := []string{"../test/hello.o"}
	symlinks := [][2]string{}
	if filepath.Base(manifestFile) == "manifest.base" {
		cmd := exec.Command("touch", "../test/hello.o")
		cmd.Run()
		manifestFile, _ = Create(config, "", infiles, outfiles, symlinks, manifestFile)

		// should find from cache
		fmt.Println(" ... 1line, 1line, 1line, 1line, 1line ... ")
		manifestFile, _ = FindManifest(config, cmdhash)
		if !utils.Exists(manifestFile) {
			t.Fail()
		}else{
			fmt.Println("Good: Found in cache")
		}
		// change 1st file
		fmt.Println(" ... 2lines, 1line, 1line, 1line, 1line ... ")
		cmd = exec.Command("cp", "../test/2lines", "../test/file1_to_test")
		cmd.Run()
		manifestFile, _ = FindManifest(config, cmdhash)
		if utils.Exists(manifestFile) {
			t.Fail()
		}else{
			fmt.Println("Good: Not found in cache")
		}
		manifestFile, _ = Create(config, "", infiles, outfiles, symlinks, manifestFile)
		fmt.Println(" ... 2lines, 2lines, 1line, 1line, 1line ... ")
		cmd = exec.Command("cp", "../test/2lines", "../test/file2_to_test")
		cmd.Run()
		manifestFile, _ = FindManifest(config, cmdhash)
		if utils.Exists(manifestFile) {
			t.Fail()
		}else{
			fmt.Println("Good: Not found in cache")
		}
		manifestFile, _ = Create(config, "", infiles, outfiles, symlinks, manifestFile)
		fmt.Println(" ... 2lines, 2lines, 2lines, 1line, 1line ... ")
		cmd = exec.Command("cp", "../test/2lines", "../test/file3_to_test")
		cmd.Run()
		manifestFile, _ = FindManifest(config, cmdhash)
		if utils.Exists(manifestFile) {
			t.Fail()
		}else{
			fmt.Println("Good: Not found in cache")
		}
		manifestFile1, _ := Create(config, "", infiles, outfiles, symlinks, manifestFile)
		fmt.Println(" ... 2lines, 2lines, 3lines, 1line, 1line ... ")
		cmd = exec.Command("cp", "../test/3lines", "../test/file3_to_test")
		cmd.Run()
		manifestFile, _ = FindManifest(config, cmdhash)
		if utils.Exists(manifestFile) {
			t.Fail()
		}else{
			fmt.Println("Good: Not found in cache")
		}
		manifestFile2, _ := Create(config, "", infiles, outfiles, symlinks, manifestFile)
		// their first 2 infiles are same, the 3rd file is different
		if filepath.Dir(manifestFile1) != filepath.Dir(manifestFile2) {
			t.Fail()
		}
		// change file3 back to 1line, should hit cache
		fmt.Println(" ... 2lines, 2lines, 1line, 1line, 1line, should hit cache ... ")
		cmd = exec.Command("cp", "../test/1line", "../test/file3_to_test")
		cmd.Run()
		manifestFile, _ = FindManifest(config, cmdhash)
		if !utils.Exists(manifestFile) {
			t.Fail()
		}
	}else{
		t.Fail()
	}
	os.RemoveAll("/tmp/cache_testTestFindManifest")
}
