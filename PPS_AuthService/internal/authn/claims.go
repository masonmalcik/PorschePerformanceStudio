package authn

import (
	"context"

	"github.com/golang-jwt/jwt/v5"
)

// Claims contains the verified Cognito access-token claims used by PPS.
type Claims struct {
	jwt.RegisteredClaims
	Username      string   `json:"username,omitempty"`
	CognitoUser   string   `json:"cognito:username,omitempty"`
	Groups        []string `json:"cognito:groups,omitempty"`
	Email         string   `json:"email,omitempty"`
	EmailVerified bool     `json:"email_verified,omitempty"`
	TokenUse      string   `json:"token_use"`
	ClientID      string   `json:"client_id"`
	Scope         string   `json:"scope,omitempty"`
}

func (c Claims) EffectiveUsername() string {
	if c.Username != "" {
		return c.Username
	}
	return c.CognitoUser
}

type claimsContextKey struct{}

func withClaims(ctx context.Context, claims Claims) context.Context {
	return context.WithValue(ctx, claimsContextKey{}, claims)
}

// ClaimsFromContext returns cryptographically verified Cognito claims.
func ClaimsFromContext(ctx context.Context) (Claims, bool) {
	claims, ok := ctx.Value(claimsContextKey{}).(Claims)
	return claims, ok
}
