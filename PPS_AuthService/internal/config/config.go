package config

import (
	"fmt"
	"os"
	"strings"
)

type Config struct {
	Port            string
	Region          string
	CognitoDomain   string
	UserPoolID      string
	ClientID        string
	Scopes          []string
	CallbackURLs    map[string]struct{}
	LogoutURLs      map[string]struct{}
	FrontendOrigins map[string]struct{}
	DynamoDBTable   string
}

func Load() (Config, error) {
	cfg := Config{
		Port:            value("PORT", "8082"),
		Region:          value("AWS_REGION", "us-east-1"),
		CognitoDomain:   strings.TrimRight(os.Getenv("COGNITO_DOMAIN"), "/"),
		UserPoolID:      os.Getenv("COGNITO_USER_POOL_ID"),
		ClientID:        os.Getenv("COGNITO_CLIENT_ID"),
		Scopes:          list(value("COGNITO_SCOPES", "openid,email,profile")),
		CallbackURLs:    set(os.Getenv("ALLOWED_CALLBACK_URLS")),
		LogoutURLs:      set(os.Getenv("ALLOWED_LOGOUT_URLS")),
		FrontendOrigins: set(os.Getenv("FRONTEND_ORIGINS")),
		DynamoDBTable:   os.Getenv("DYNAMODB_TABLE"),
	}
	if cfg.CognitoDomain == "" || cfg.UserPoolID == "" || cfg.ClientID == "" || cfg.DynamoDBTable == "" {
		return Config{}, fmt.Errorf("COGNITO_DOMAIN, COGNITO_USER_POOL_ID, COGNITO_CLIENT_ID, and DYNAMODB_TABLE are required")
	}
	if len(cfg.CallbackURLs) == 0 || len(cfg.LogoutURLs) == 0 {
		return Config{}, fmt.Errorf("ALLOWED_CALLBACK_URLS and ALLOWED_LOGOUT_URLS must contain at least one URL")
	}
	return cfg, nil
}

func (c Config) Issuer() string {
	return "https://cognito-idp." + c.Region + ".amazonaws.com/" + c.UserPoolID
}
func (c Config) JWKSURL() string { return c.Issuer() + "/.well-known/jwks.json" }

func value(name, fallback string) string {
	if current := strings.TrimSpace(os.Getenv(name)); current != "" {
		return current
	}
	return fallback
}

func list(raw string) []string {
	values := []string{}
	for _, item := range strings.Split(raw, ",") {
		if item = strings.TrimSpace(item); item != "" {
			values = append(values, item)
		}
	}
	return values
}

func set(raw string) map[string]struct{} {
	values := map[string]struct{}{}
	for _, item := range list(raw) {
		values[item] = struct{}{}
	}
	return values
}
