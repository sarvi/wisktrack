package config

import (
	"fmt"
	"testing"
)

func TestIsUpper(t *testing.T) {
	fmt.Println("Test IsUpper Config")
}

func TestIsLower(t *testing.T) {
	fmt.Println("Test IsLower Config")
	t.Fail()
}
