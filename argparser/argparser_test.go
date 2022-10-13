package argparser

import (
	"fmt"
	"testing"
)

func TestIsUpper(t *testing.T) {
	fmt.Println("Test IsUpper ArgParser")
}

func TestIsLower(t *testing.T) {
	fmt.Println("Test IsLower ArgParser")
	t.Fail()
}
