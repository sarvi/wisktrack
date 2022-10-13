# wiskcache/cache 
put cache.go and cache_test.go or other files for package cache
To run test:
go test -v cache.go cache_test.go


func FindManifest(config config.Config, cmdhash string, infile []string)(string, error)
func Create(config config.Config, infile []string, outfile []string, manifestfile string)(error)
func CopyOut(config config.Config, manifestfile string)(error)
func Verify(config config.Config, manifestfile string)(bool)

infiles and outfiles are relative path to config.BaseDir
e.g.
    config := config.Config{CacheBaseDir: "/nobackup/ldu/test_wiskcache", BaseDir: "/ws/ldu-sjc/temp6_ci"}

    manifestFile, _ := cache.FindManifest(config, "test1", []string{"toothless/src/manage.py", "toothless/src/pytest.ini"})
    if !utils.Exists(manifestFile){
       // create manifest, copy outputfiles to cachedir
       cache.Create(config, []string{"toothless/src/manage.py", "toothless/src/pytest.ini"}, []string{"toothless/src/manage.pyc"}, manifestFile)
    }else{
       // manifestFile matched, copy from cache onto worksapce
       cache.CopyOut(config, manifestFile)
       // or verify, true if all matched, false otherwise
       cache.Verify(config, manifestFile)
    }
