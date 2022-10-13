package config

import (
	"fmt"
	"io/ioutil"

	"gopkg.in/yaml.v2"
)

//Tool specific config values
type Tool struct {
	Match  string   `yaml:"Match"`
	Envars []string `yaml:"Envars"`
}

//Config structure declaration
type Config struct {
	ToolIdx      int      `yaml:"ToolIdx"`
	UserName     string   `yaml:"UserName"`
	Mode         string   `yaml:"Mode"`
	BaseDir      string   `yaml:"BaseDir"`
	WiskTrackLib string   `yaml:"WiskTrackLib"`
	Envars       []string `yaml:"Envars"`
	Tools        []Tool   `yaml:"Tool"`
	CacheBaseDir string   `yaml:"CacheBaseDir"`
}

//Parseconfig function to parse the config file
func Parseconfig(InputconfigFile string) (ConfigValues Config) {
	ConfigFile, err := ioutil.ReadFile(InputconfigFile)
	if err != nil {
		fmt.Printf("Error reading Wiskcache configure file: %s\n", err)
	}
	err = yaml.Unmarshal(ConfigFile, &ConfigValues)
	if err != nil {
		fmt.Printf("Error parsing Wiskcache configure file: %s\n", err)
	}
	return
}
