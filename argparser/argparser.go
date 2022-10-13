package argparser

import (
	"config"
	"flag"
	"fmt"
	"log"
	"os"
	"os/user"
	"path/filepath"
	"strings"
)

// contains checks if a string is present in a slice
func contains(s []string, str string) bool {
	for _, v := range s {
		if v == str {
			return true
		}
	}

	return false
}

//ArgParse funtion to parse arguments
func ArgParse() (ConfigValues config.Config, CommandLine []string) {
	var defaultConfigFile string = os.Getenv("WISK_CONFIG")
	var defaultBaseDir string = os.Getenv("WISK_WSROOT")
	args := []string{}
	cmdargi := -1
	cmdargj := -1
	for i, v := range os.Args {
		if v == "---" {
			cmdargi = i + 1
			for j, w := range os.Args[cmdargi:] {
				if strings.Contains(w, "=") {
					pair := strings.SplitN(w, "=", 2)
					os.Setenv(pair[0], pair[1])
				} else {
					cmdargj = cmdargi + j
					break
				}
			}
			break
		}
		args = append(args, v)
	}
	if cmdargi < 0 || cmdargi >= len(os.Args) || cmdargj < 0 {
		log.Fatalf("No command-to-cache provided. wiskcache <wiskcache-options> --- command-to-cacche")
	}

	CommandLine = os.Args[cmdargj:]
	os.Args = os.Args[:cmdargi-1]
	if defaultBaseDir == "" {
		defaultBaseDir, _ = os.Getwd()
	}
	var InputconfigFile string
	baseDir := flag.String("base_dir", defaultBaseDir, "Wisk will rewrite absolute paths beginning with base_dir into paths relative to the current working directory")
	if defaultConfigFile == "" {
		defaultConfigFile = filepath.Join(*baseDir, "wisk/config/wiskdeps_config.yaml")
	}
	flag.StringVar(&InputconfigFile, "config", defaultConfigFile, "Wisk configure file location")
	flag.Parse()

	//Parsing config file to get Configvalues instance
	if InputconfigFile != "" {
		_, err := os.Stat(InputconfigFile)
		if err == nil {
			ConfigValues = config.Parseconfig(InputconfigFile)
		} else {
			fmt.Printf("Config file %s does not exist", InputconfigFile)
			os.Exit(1)
		}
	}

	//ToolIndex default value set to -1
	ConfigValues.ToolIdx = -1

	user, err := user.Current()
	if err != nil {
		panic(err)
	}
	ConfigValues.UserName = user.Username

	if ConfigValues.WiskTrackLib == "" {
		libpath, err := os.Executable()
		if err != nil {
			fmt.Println("Cannot Locate Wisk Track Library")
			log.Fatal(err)
		}
		libpath, err = filepath.EvalSymlinks(libpath)
		if err != nil {
			log.Fatal(err)
		}
		libpath = filepath.Join(filepath.Dir(filepath.Dir(libpath)), "${LIB}", "libwisktrack.so")
		ConfigValues.WiskTrackLib = libpath
	}

	if *baseDir != "" {
		ConfigValues.BaseDir = *baseDir
	}

	if ConfigValues.CacheBaseDir == "" {
		log.Fatalf("CacheBaseDir is not set in %s", InputconfigFile)
	}

	if len(ConfigValues.Envars) == 0 {
		ConfigValues.Envars = []string{"CWD"}
	}

	return

}
