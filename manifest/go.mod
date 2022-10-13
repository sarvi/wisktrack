module manifest

go 1.16

replace utils => ../utils
replace config => ../config

require (
	lukechampine.com/blake3 v1.1.5
	utils v0.0.0-00010101000000-000000000000
)
