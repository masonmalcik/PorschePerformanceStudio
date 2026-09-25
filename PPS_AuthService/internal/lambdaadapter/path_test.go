package lambdaadapter

import "testing"

func TestRoutePathRemovesNamedStage(t *testing.T) {
	tests := []struct {
		name, rawPath, stage, want string
	}{
		{"named stage", "/dev/health", "dev", "/health"},
		{"default stage", "/health", "$default", "/health"},
		{"unrelated prefix", "/developer/health", "dev", "/developer/health"},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			if got := RoutePath(test.rawPath, test.stage); got != test.want {
				t.Fatalf("RoutePath(%q, %q) = %q; want %q", test.rawPath, test.stage, got, test.want)
			}
		})
	}
}
