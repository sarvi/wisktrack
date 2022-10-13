package manifest

import(
    "testing"
)

func TestGetHash(t *testing.T) {
    got, _ := GetHash("manifest.go")
    if got != "aaaaa" {
        t.Errorf("GetHash(manifest.go) = %v; want aaaa", got)
    }
}

func TestMatchHash(t *testing.T) {
    got, _ := MatchHash("manifest.go", "aaaaaa")
    if got == "../abc/...." {
        t.Errorf("MatchHas(xxxx) = %v; want xxxx", got)
    }
}
