package manifest

import (
    "os"
    "io"
    "fmt"
    "errors"
    "utils"
    "lukechampine.com/blake3"
    "encoding/json"
    "io/ioutil"
    "path/filepath"
    "sync"
)

type FileManifest struct{
    InputFile [][]string `json:"inputfile"`
    OutputFile [][]string `json:"outputfile"`
    SymLink [][]string `json:"symlink"`
    LogFile string `json:"logfile"`
}

func GetHash(file string)(string, error){
    if utils.Exists(file){
        f, err := os.Open(file)
        if err != nil {
            return "", err
        }
        defer f.Close()
        h := blake3.New(32, nil)
        if _, err := io.Copy(h, f); err != nil {
            return "", err
        }
        return fmt.Sprintf("%x", h.Sum(nil)), nil
    }else{
        return "", errors.New(fmt.Sprintf("%v does not exist", file))
    }
}

func MatchHash(file string, hash string)(string, error){
    if utils.Exists(file){
        new_hash, err := GetHash(file) 
        if err != nil {
            return "", err
        }
        if hash == new_hash {
            return "", nil
        }else{
            return new_hash, nil
        }
    }else{
        return "", errors.New(fmt.Sprintf("%v does not exist", file))
    }
}

func GenerateManifest(logFile string, inputFileList []string, outputFileList []string, symLinks [][2]string, baseDirOfWorkspace string)(FileManifest){
    manifest := FileManifest{InputFile:[][]string{}, OutputFile:[][]string{}, SymLink:[][]string{}, LogFile:""}
    if logFile != "" {
        manifest.LogFile = filepath.Base(logFile)
    }else{
        manifest.LogFile = ""
    }

    hashOfFile := make(map[string]string)
    allFiles := []string{}
    allFiles = append(allFiles, inputFileList...)
    allFiles = append(allFiles, outputFileList...)
    var wg sync.WaitGroup
    var mutex = &sync.Mutex{}
    for _, file := range allFiles{
        wg.Add(1)
        go func(file string){
            defer wg.Done()
            fullpath := file
            if !filepath.IsAbs(fullpath){
                fullpath = filepath.Join(baseDirOfWorkspace, file)
            }
            hash, err := GetHash(fullpath)
            if err == nil{
                mutex.Lock()
                hashOfFile[file] = hash
                mutex.Unlock()
            }
        }(file)
    }
    wg.Wait()

    for _, file := range inputFileList{
        manifest.InputFile = append(manifest.InputFile, []string{file, hashOfFile[file]})
    }
    for _, file := range outputFileList{
        if !filepath.IsAbs(file){
             manifest.OutputFile = append(manifest.OutputFile, []string{file, hashOfFile[file]})
        }
    }

    for _, symlink := range symLinks{
        manifest.SymLink = append(manifest.SymLink, []string{symlink[0], symlink[1]})
    }

    return manifest
}

func ReadManifest(manifestFile string)(FileManifest, error){
    var manifest FileManifest
    if !utils.Exists(manifestFile){
        return manifest, errors.New(fmt.Sprintf("%v does not exist.", manifestFile))
    }else{
        data, err := ioutil.ReadFile(manifestFile)
        if err != nil{
            return manifest, errors.New(fmt.Sprintf("Cannot read %v", manifestFile))
        }
        json.Unmarshal(data, &manifest)
        return manifest, nil 
    }
}

func SaveManifestFile(manifest FileManifest, manifestWithHash string, manifestFile string)(string, error){
    // manifestFile is retrieved from cache.FindManifest

    var err error

    if utils.Exists(manifestWithHash) {
        return manifestWithHash, nil
    }
    baseOfCacheDir := filepath.Dir(filepath.Dir(manifestFile))

    if filepath.Base(manifestFile) == "manifest.base" {
        manifestFile = filepath.Join(baseOfCacheDir, "manifest.base")
    }

    cacheDir := filepath.Dir(manifestFile)
    if !utils.Exists(cacheDir){
        err = os.MkdirAll(cacheDir, 0775)
        if err != nil{
            return manifestFile, err
        }
    } 
    jsondata, _ := json.MarshalIndent(manifest, "", " ")
    err = ioutil.WriteFile(manifestWithHash, jsondata, 0664)
    if err != nil{
        return manifestFile, err
    }

    // make symlink
    relativePath := ""
    if filepath.Base(manifestFile) == "manifest.base" {
        relativePath, _ = utils.RelativePath(baseOfCacheDir, manifestWithHash)
    }else{
        relativePath, _ = utils.RelativePath(cacheDir, manifestWithHash)
    }
    os.Symlink(relativePath, manifestFile)
    return manifestFile, nil
}
