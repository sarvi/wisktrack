package exec

import (
	"bufio"
	"config"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"utils"
)

func ParseWiskTrackFile(trackfile string) (infiles []string, outfiles []string, canbecached bool) {
	canbecached = true
	file, err := os.Open(trackfile)
	if err != nil {
		return
	}
	defer file.Close()

	inmap := map[string]string{}
	outmap := map[string]string{}
	var jsondata []interface{}
	var line string
	var parts []string
	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line = scanner.Text()
		parts = strings.SplitN(line, " ", 3)
		if parts[1] == "READS" {
			json.Unmarshal([]byte(parts[2]), &jsondata)
			// fmt.Println("READS: ", jsondata)
			if opfile, ok := jsondata[0].(string); ok {
				if strings.HasPrefix(opfile, "/dev/") || strings.HasPrefix(opfile, "/proc/") {
					continue
				}
				if _, ok := outmap[opfile]; !ok {
					if _, ok := inmap[opfile]; !ok {
						inmap[opfile] = ""
						infiles = append(infiles, opfile)
					}
				}
			} else {
				panic(ok)
			}
		} else if parts[1] == "WRITES" {
			json.Unmarshal([]byte(parts[2]), &jsondata)
			// fmt.Println("WRITES: ", jsondata)
			if opfile, ok := jsondata[0].(string); ok {
				if strings.HasPrefix(opfile, "/dev/") {
					continue
				}
				if _, ok := inmap[opfile]; ok {
					canbecached = false
					fmt.Printf("WARNING: Input file %s is being modified. Not Cacheable. Suggest rewriting command to separate files read and written by tool", opfile)
				} else {
					if _, ok := outmap[opfile]; !ok {
						outmap[opfile] = ""
						outfiles = append(outfiles, opfile)
					}
				}
			} else {
				panic(ok)
			}
		} else if parts[1] == "RENAMES" {
			json.Unmarshal([]byte(parts[2]), &jsondata)
			if opfile, ok := jsondata[0].(string); ok {
				if _, ok := outmap[opfile]; ok {
					delete(outmap, opfile)
					outfiles = utils.Remove(outfiles, opfile)
					if opfile, ok := jsondata[1].(string); ok {
						if _, ok := outmap[opfile]; !ok {
							outmap[opfile] = ""
							outfiles = append(outfiles, opfile)
						} else {
							panic(ok)
						}
					} else {
						panic(ok)
					}
				} else {
					panic(ok)
				}
			} else {
				panic(ok)
			}
		}
	}
	// fmt.Println("Infiles: ", infiles)
	// fmt.Println("Outfiles: ", outfiles)
	return
}

func RunCmd(conf config.Config, cmdhash string, cmd []string) (exitcode int, logfile string, infiles []string, outfiles []string, symlinks [][2]string, canbecached bool) {
	fmt.Println("Executing: ", cmd)
	fmt.Println("Hash: ", cmdhash)
	logfile = fmt.Sprintf("/tmp/%s/wisktrack/wiskcachecmdrun.%s.log", conf.UserName, cmdhash)
	trackfile := fmt.Sprintf("/tmp/%s/wisktrack/wisktrack.%s.file", conf.UserName, cmdhash)
	os.Remove(trackfile)
	if !utils.Exists(filepath.Dir(logfile)) {
		os.MkdirAll(filepath.Dir(logfile), 0775)
	}

	out, err := os.Create(logfile)
	if err != nil {
		panic(err)
	}
	defer out.Close()
	ro, wo := io.Pipe()
	defer wo.Close()
	re, we := io.Pipe()
	defer we.Close()
	command := exec.Command(cmd[0], cmd[1:]...)
	command.Stdout = io.MultiWriter(wo, out)
	command.Stderr = io.MultiWriter(we, out)
	go io.Copy(os.Stdout, ro)
	go io.Copy(os.Stderr, re)
	command.Env = append(
		os.Environ(),
		fmt.Sprintf("LD_PRELOAD=%s", conf.WiskTrackLib),
		"WISK_CONFIG=",
		"WISK_TRACE=%s/wisktrace.log",
		fmt.Sprintf("WISK_TRACK=/tmp/%s/wisktrack/wisktrack.%s.file", conf.UserName, cmdhash),
		fmt.Sprintf("WISK_WSROOT=%s", conf.BaseDir),
		// fmt.Sprintf("WISK_TRACE=%s/wisktrace.log", conf.BaseDir),
	)
	fmt.Println("Run Trackfile: ", trackfile)
	err = command.Start()
	if err != nil {
		panic(err)
	}
	command.Wait()
	if !utils.Exists(trackfile) {
		log.Fatalf("Cannot find trackfile: %s.\nWiskTrack Preload Library probably did not get load or initialized.\nWrong LD_PRELOAD=%s\nSet WiskTrackLib in config file", trackfile, conf.WiskTrackLib)
	}
	exitcode = 0
	infiles, outfiles, canbecached = ParseWiskTrackFile(trackfile)
	fmt.Println("Run Logfile: ", logfile)
	fmt.Println("Run Infiles: ", infiles)
	fmt.Println("Run Outfile: ", outfiles)
	fmt.Println("Run Symlink: ", symlinks)
	fmt.Println("Can be Cached: ", canbecached)
	return
}
