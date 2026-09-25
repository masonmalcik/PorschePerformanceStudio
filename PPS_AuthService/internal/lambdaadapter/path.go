package lambdaadapter

import "strings"

func RoutePath(rawPath, stage string) string {
	prefix := "/" + strings.Trim(stage, "/")
	if stage != "" && stage != "$default" && strings.HasPrefix(rawPath, prefix+"/") {
		return strings.TrimPrefix(rawPath, prefix)
	}
	return rawPath
}
