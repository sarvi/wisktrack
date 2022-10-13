module exec

go 1.16

replace utils => ../utils

replace config => ../config

require (
	config v0.0.0-00010101000000-000000000000
	utils v0.0.0-00010101000000-000000000000
)
