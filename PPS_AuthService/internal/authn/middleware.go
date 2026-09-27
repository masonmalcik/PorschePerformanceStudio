package authn

import (
	"encoding/json"
	"errors"
	"log/slog"
	"net/http"
	"strings"
)

type Middleware struct {
	verifier Verifier
	logger   *slog.Logger
}

func NewMiddleware(verifier Verifier, logger *slog.Logger) *Middleware {
	return &Middleware{verifier: verifier, logger: logger}
}

func (m *Middleware) Authenticate(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		scheme, rawToken, found := strings.Cut(strings.TrimSpace(r.Header.Get("Authorization")), " ")
		if !found || !strings.EqualFold(scheme, "Bearer") || strings.TrimSpace(rawToken) == "" || len(rawToken) > 16<<10 {
			writeAuthError(w, http.StatusUnauthorized, "unauthorized", "a valid Cognito access token is required")
			return
		}
		claims, err := m.verifier.Verify(r.Context(), strings.TrimSpace(rawToken))
		if errors.Is(err, ErrJWKSUnavailable) {
			m.logger.Error("Cognito JWKS unavailable", "error", err)
			w.Header().Set("Retry-After", "1")
			writeAuthError(w, http.StatusServiceUnavailable, "identity_provider_unavailable", "token verification is temporarily unavailable")
			return
		}
		if err != nil {
			writeAuthError(w, http.StatusUnauthorized, "unauthorized", "a valid Cognito access token is required")
			return
		}
		next.ServeHTTP(w, r.WithContext(withClaims(r.Context(), claims)))
	})
}

func writeAuthError(w http.ResponseWriter, status int, code, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(map[string]any{"error": map[string]string{"code": code, "message": message}})
}
