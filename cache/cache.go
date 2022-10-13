package cache

import(
    "fmt"
    "config"
    "path/filepath"
    "manifest"
    "utils"
    "os"
    "os/exec"
    "strings"
    "io"
    "sync"
)

func Greet() {
    fmt.Println("Hello World Cache!")
}

func FindManifest(config config.Config, cmdhash string)(string, error){
    // return manifestFile which could exist or not
    
    cacheDir := filepath.Join(config.CacheBaseDir, cmdhash)
    if !utils.Exists(cacheDir){
        err := os.MkdirAll(cacheDir, 0775)
        if err != nil{
            return "", err
        }
    }
    manifestFile := ""
    if utils.Exists(filepath.Join(cacheDir, "manifest.base")){
        manifestFile = filepath.Join(cacheDir, "manifest.base")
    }else{
        allmanifestfiles, _ := utils.ReadDir(cacheDir, "manifest.")
        if len(allmanifestfiles) == 0 {
           return filepath.Join(cacheDir, "partial.base", "manifest.base"), nil
        }else{
            manifestFile = filepath.Join(cacheDir, allmanifestfiles[0])
            // if symlink manifest.base is gone, create a new one
            relativePath, _ := utils.RelativePath(cacheDir, manifestFile)
            os.Symlink(relativePath, filepath.Join(cacheDir, "manifest.base"))
        }
    }
    mismatch := false
    for{
        manif := manifest.FileManifest{InputFile:[][]string{}, OutputFile:[][]string{}}
        fmt.Printf("Reading %s\n", manifestFile)
        manifestFromFile, _ := manifest.ReadManifest(manifestFile)
        for _, inputFile := range manifestFromFile.InputFile{
            fullpath := inputFile[0]
            if !filepath.IsAbs(fullpath){
                fullpath = filepath.Join(config.BaseDir, fullpath)
            }
            hash, _ := manifest.GetHash(fullpath)
            manif.InputFile = append(manif.InputFile, []string{inputFile[0], hash})
        }
        if utils.Exists(filepath.Join(cacheDir, "manifest." + utils.HashOfFileAndHash(manif.InputFile))) {
            return filepath.Join(cacheDir, "manifest." + utils.HashOfFileAndHash(manif.InputFile)), nil
        }

        for inputIndex, inputFile := range manifestFromFile.InputFile{
            if manif.InputFile[inputIndex][0] != inputFile[0] ||
               manif.InputFile[inputIndex][1] != inputFile[1]{
               listOfCurrentFile := [][]string{}
               listOfCurrentFile = append(listOfCurrentFile, []string{manif.InputFile[inputIndex][0], manif.InputFile[inputIndex][1]})
                if inputIndex == 0 {
                    manifestFile = filepath.Join(cacheDir, "partial.base",
                                                 fmt.Sprintf("manifest.%v", utils.HashOfFileAndHash(listOfCurrentFile)))
                }else{
                    manifestFile = filepath.Join(cacheDir, "partial." + utils.HashOfFileAndHash(manif.InputFile[:inputIndex]),
                                                 fmt.Sprintf("manifest.%v", utils.HashOfFileAndHash(listOfCurrentFile)))
                }
                mismatch = true
                break
            }
        }
        if mismatch == false{
            return manifestFile, nil
        }

        if !utils.Exists(manifestFile){
            return manifestFile, nil
        }
        manifestFromFile, _ = manifest.ReadManifest(manifestFile)
        mismatch = false
    }
    return "", nil
}

func Create(config config.Config, logFile string, inFile []string, outFile []string, symLinks [][2]string, manifestfile string)(string, error){
    // create manifest file and copy outputfiles to cache

    var err error
    // manifestfile is retrieved from FindManifest
    // create manifest file
    infile, _ := utils.ConverFilesToRelativePath(config, inFile)
    outfile, _ := utils.ConverFilesToRelativePath(config, outFile)
    // if an output file is in inputFileList as well, remove it from inputFileList
    infile = utils.RemoveFromArray(infile, outfile)
    manifestdata := manifest.GenerateManifest(logFile, infile, outfile, symLinks, config.BaseDir)
    hashOfAllInFiles := utils.HashOfFileAndHash(manifestdata.InputFile)
    baseOfCacheDir := filepath.Dir(filepath.Dir(manifestfile))

    // copy outputfiles to cache
    dirOfCachedOutputFiles := filepath.Join(baseOfCacheDir, "content." + hashOfAllInFiles)
    if !utils.Exists(dirOfCachedOutputFiles){
        err = os.MkdirAll(dirOfCachedOutputFiles, 0775)
        if err != nil{
            return manifestfile, err
        }
    }
    var wg sync.WaitGroup
    for _, ofile := range outfile{
        // full path means it's not a file in workspace
        if filepath.IsAbs(ofile){
            continue
        }
        target := filepath.Join(dirOfCachedOutputFiles, strings.Replace(ofile, "/", ".", -1))
        source := filepath.Join(config.BaseDir, ofile)
        fmt.Printf("Copying %v to %v\n", source, target)
        wg.Add(1)
        go func(src string, tgt string){
            defer wg.Done()
            cpCmd := exec.Command("cp", src, tgt)
            cperr := cpCmd.Run()
            if cperr != nil{
                err = cperr
            }
        }(source, target)
    }
    wg.Wait()
    if err != nil{
        return manifestfile, err
    }

    if logFile != "" && utils.Exists(logFile){
        tgtLogfile := filepath.Join(dirOfCachedOutputFiles, filepath.Base(logFile))
        cpCmd := exec.Command("cp", logFile, tgtLogfile)
        fmt.Printf("Copying %v to %v\n", logFile, tgtLogfile)
        err = cpCmd.Run()
        if err != nil{
            return manifestfile, err
        }
    }

    manifestfile, _ = manifest.SaveManifestFile(manifestdata, filepath.Join(baseOfCacheDir, "manifest." + hashOfAllInFiles),  manifestfile)
    manifestfile, err = filepath.EvalSymlinks(manifestfile)
    return manifestfile, err
}

func CopyOut(config config.Config, manifestFile string)(error){
    // copy from cache
    var err error
    manifestFile, _ = filepath.EvalSymlinks(manifestFile)
    dirOfCachedOutputFiles := filepath.Join(filepath.Dir(manifestFile),
                                            strings.Replace(filepath.Base(manifestFile), "manifest.", "content.", 1))
    manifestdata, _ := manifest.ReadManifest(manifestFile)
    var wg sync.WaitGroup
    for _, outputFile := range manifestdata.OutputFile{
        // if outputFile is abs path, it's not a file in workspace then
        if filepath.IsAbs(outputFile[0]){
            continue
        }
        srcFile := filepath.Join(dirOfCachedOutputFiles, strings.Replace(outputFile[0], "/", ".", -1))
        tgtFile := filepath.Join(config.BaseDir, outputFile[0])
        dirOfTgt := filepath.Dir(tgtFile)
        if !utils.Exists(dirOfTgt){
            err = os.MkdirAll(dirOfTgt, 0775)
            if err != nil{
                break
            }
        }
        fmt.Printf("Copying %v to %v\n", srcFile, tgtFile)
        wg.Add(1)
        go func(srcFile string, tgtFile string){
            defer wg.Done()
            cpCmd := exec.Command("cp", srcFile, tgtFile)
            cperr := cpCmd.Run()
            if cperr != nil{
                err = cperr
            }
        }(srcFile, tgtFile)
    }
    wg.Wait()
    if err != nil{
        return err
    }

    for _, symLink := range manifestdata.SymLink{
        wg.Add(1)
        go func(symLink []string){
            defer wg.Done()
            os.Symlink(symLink[1], filepath.Join(config.BaseDir, symLink[0]))
        }(symLink)
    }
    wg.Wait()

    // print out log file
    if manifestdata.LogFile != ""{
        logFile := filepath.Join(dirOfCachedOutputFiles, manifestdata.LogFile)
        if utils.Exists(logFile){
            file, err := os.Open(logFile)
            if err != nil{
                fmt.Printf("Failed to open %v\n", logFile)
            }else{
                _, err = io.Copy(os.Stdout, file)
                if err != nil {
                    fmt.Printf("io.Copy failed  %v\n", err)
                }
            }
        }
    }
    return nil
}

func Verify(config config.Config, manifestFile string)(bool){
    manifestdata, _ := manifest.ReadManifest(manifestFile)
    matched := true
    for _, outputFile := range manifestdata.OutputFile{
        fullpath := outputFile[0]
        hash := outputFile[1]
        if !filepath.IsAbs(fullpath){
            fullpath = filepath.Join(config.BaseDir, fullpath)
        }
        hashOfFileInWorkspace, _ := manifest.GetHash(fullpath)
        fmt.Printf("Comparing %v ...\n", outputFile)
        if hash != hashOfFileInWorkspace{
            fmt.Printf("%v is not matched, hash: %v, hashInWorkspace %v\n", fullpath, hash, hashOfFileInWorkspace)
            matched = false 
        }
    }
    return matched
}
