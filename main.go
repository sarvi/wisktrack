package main

import (
	"argparser"
	"config"
	"exec"
	"fmt"
	"strings"

	//"manifest"
	"cache"
	"utils"
	"whash"
)

func main() {
	var ConfigValues config.Config
	var CommandtoExec []string

	ConfigValues, CommandtoExec = argparser.ArgParse()

	cmdexeced := false
	infiles := []string{}
	outfiles := []string{}
	symlinks := [][2]string{}
	logfile := ""
	manifestFile := ""
	env := utils.GetEnvironMap()
	_, cmdhash, _ := whash.CommandHash(ConfigValues, env, CommandtoExec)
	if strings.HasPrefix(ConfigValues.Mode, "read") || ConfigValues.Mode == "verify" {
		manifestFile, _ = cache.FindManifest(ConfigValues, cmdhash)
		if utils.Exists(manifestFile) && ConfigValues.Mode != "verify" {
			fmt.Printf("Found manifest: %v, copying out from cache ...\n", manifestFile)
			cache.CopyOut(ConfigValues, manifestFile)
			fmt.Println("Done!")
		}
	}
	if !utils.Exists(manifestFile) || ConfigValues.Mode == "verify" || ConfigValues.Mode == "learn" {
		cmdexeced = true
		_, logfile, infiles, outfiles, symlinks, _ = exec.RunCmd(ConfigValues, cmdhash, CommandtoExec)
	}
	if strings.Contains(ConfigValues.Mode, "write") && cmdexeced {
		manifestFile, _ = cache.Create(ConfigValues, logfile, infiles, outfiles, symlinks, manifestFile)
		fmt.Printf("\nCreated manifest: %v, copied output to cache\n", manifestFile)
	}
	if ConfigValues.Mode == "verify" && cmdexeced {
		if utils.Exists(manifestFile) {
			fmt.Println("Verifying ...")
			if cache.Verify(ConfigValues, manifestFile) {
				fmt.Println("All Matched.")
			}
		} else {
			fmt.Printf("%v is not found and can't verify\n", manifestFile)
		}
	}
	if ConfigValues.Mode == "learn" && cmdexeced {
		fmt.Println("Learn Mode, executing the command second time, collect learning data ...")
		_, logfile, infiles, outfiles, symlinks, _ = exec.RunCmd(ConfigValues, cmdhash, CommandtoExec)
	}
}
