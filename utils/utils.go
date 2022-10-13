package utils

import (
	"config"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"lukechampine.com/blake3"
)

func Remove(s []string, r string) []string {
	for i, v := range s {
		if v == r {
			return append(s[:i], s[i+1:]...)
		}
	}
	return s
}

func GetEnvironMap() (result map[string]string) {
	result = map[string]string{}
	for _, v := range os.Environ() {
		spl := strings.SplitN(v, "=", 2)
		result[spl[0]] = spl[1]
	}
	return
}

func Exists(name string) bool {
	if _, err := os.Stat(name); err != nil {
		if os.IsNotExist(err) {
			return false
		}
	}
	return true
}

func RelativePath(basePath string, tgtPath string) (string, error) {
	var relpath = tgtPath
	var err error
	if filepath.IsAbs(tgtPath) {
		relpath, err = filepath.Rel(basePath, tgtPath)
		if err != nil {
			return relpath, err
		}
	}
	// check if path exists
	fullpath := filepath.Join(basePath, relpath)
	if !Exists(fullpath) {
		return "", errors.New(fmt.Sprintf("%v does not exist", fullpath))
	}
	return relpath, nil
}

func ConverFilesToRelativePath(config config.Config, infile []string) ([]string, error) {
	var err error
	outfile := make([]string, len(infile))
	for i := 0; i < len(infile); i++ {
		// a workaround
		infile[i] = strings.Replace(infile[i], "\"", "", -1)
		if filepath.IsAbs(infile[i]) && strings.HasPrefix(infile[i], config.BaseDir) {
			outfile[i], err = RelativePath(config.BaseDir, infile[i])
		} else {
			outfile[i] = infile[i]
		}
	}
	return outfile, err
}

func RemoveFromArray(list1 []string, list2 []string) []string {
	// remove list1's element if it's in list2
	list1map := make(map[string]bool)
	for _, key := range list1 {
		list1map[key] = true
	}
	list2map := make(map[string]bool)
	for _, key := range list2 {
		list2map[key] = true
	}
	for key, _ := range list1map {
		if _, ok := list2map[key]; ok {
			delete(list1map, key)
		}
	}
	keys := []string{}
	for _, key := range list1 {
		if _, ok := list1map[key]; ok {
			keys = append(keys, key)
		}
	}
	return keys
}


func ReadDir(dirname string, startwith string)([]string, error) {
	dir, err := os.Open(dirname)
	if err != nil {
		fmt.Printf("Failed opening directory: %s\n", err)
		return nil, err
	}
	defer dir.Close()

	list, _ := dir.Readdirnames(0) // 0 to read all files and folders
	result := []string{}
	for _, name := range list {
		if startwith != "" && strings.HasPrefix(name, startwith){
			result = append(result, name)
		}
	}
	return result, nil
}

func HashOfFileAndHash(filelist [][]string)(string) {
    filehashlist := []string{}
    for _, file := range filelist{
        filehashlist = append(filehashlist, file[0], file[1])
    }
    h := blake3.New(32, nil)
    filehash := strings.Join(filehashlist, " ")
    h.Write([]byte(filehash))
    return fmt.Sprintf("%x", h.Sum(nil))
}
