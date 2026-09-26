package httpapi

import (
	"context"
	"encoding/json"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/porsche-performance-studio/pps-auth-service/internal/config"
	"github.com/porsche-performance-studio/pps-auth-service/internal/profile"
)

type memoryProfiles struct{ value profile.Profile }

func (m *memoryProfiles) GetOrCreate(_ context.Context, identity profile.Identity) (profile.Profile, error) {
	if m.value.UserID == "" {
		m.value = profile.Profile{UserID: identity.UserID, Email: identity.Email, EmailVerified: identity.EmailVerified, Issuer: identity.Issuer, CreatedAt: time.Unix(1, 0), UpdatedAt: time.Unix(1, 0)}
	}
	return m.value, nil
}
func (m *memoryProfiles) Update(_ context.Context, identity profile.Identity, update profile.Update) (profile.Profile, error) {
	m.value = profile.Profile{UserID: identity.UserID, Email: identity.Email, EmailVerified: identity.EmailVerified, Issuer: identity.Issuer, DisplayName: update.DisplayName, Locale: update.Locale, Timezone: update.Timezone}
	return m.value, nil
}

func testConfig(domain string) config.Config {
	return config.Config{Port: "8082", Region: "us-east-1", CognitoDomain: domain, UserPoolID: "pool", ClientID: "client", Scopes: []string{"openid", "pps-api/profile.read"}, CallbackURLs: map[string]struct{}{"http://localhost:4321/auth/callback": {}}, LogoutURLs: map[string]struct{}{"http://localhost:4321/": {}}, FrontendOrigins: map[string]struct{}{"http://localhost:4321": {}}, DynamoDBTable: "profiles"}
}

func TestAuthorizeBuildsPKCERedirect(t *testing.T) {
	handler := New(testConfig("https://example.auth.us-east-1.amazoncognito.com"), &memoryProfiles{}, http.DefaultClient, slog.New(slog.NewTextHandler(io.Discard, nil)))
	verifier := strings.Repeat("a", 43)
	request := httptest.NewRequest(http.MethodGet, "/oauth/authorize?redirect_uri=http%3A%2F%2Flocalhost%3A4321%2Fauth%2Fcallback&state=1234567890abcdef&code_challenge="+PKCEChallenge(verifier), nil)
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusFound {
		t.Fatalf("expected 302, got %d: %s", response.Code, response.Body.String())
	}
	location := response.Header().Get("Location")
	for _, expected := range []string{"response_type=code", "code_challenge_method=S256", "client_id=client"} {
		if !strings.Contains(location, expected) {
			t.Errorf("location missing %q: %s", expected, location)
		}
	}
}

func TestAuthorizeRejectsUnlistedRedirect(t *testing.T) {
	handler := New(testConfig("https://example.invalid"), &memoryProfiles{}, http.DefaultClient, slog.New(slog.NewTextHandler(io.Discard, nil)))
	request := httptest.NewRequest(http.MethodGet, "/oauth/authorize?redirect_uri=https%3A%2F%2Fevil.example&state=1234567890abcdef&code_challenge="+strings.Repeat("a", 43), nil)
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusBadRequest {
		t.Fatalf("expected 400, got %d", response.Code)
	}
}

func TestMeUsesCognitoSubjectAsProfileKey(t *testing.T) {
	cognito := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get("Authorization") != "Bearer token" {
			t.Error("bearer token not forwarded")
		}
		_ = json.NewEncoder(w).Encode(map[string]any{"sub": "user-123", "email": "owner@example.com", "email_verified": true})
	}))
	defer cognito.Close()
	repo := &memoryProfiles{}
	handler := New(testConfig(cognito.URL), repo, cognito.Client(), slog.New(slog.NewTextHandler(io.Discard, nil)))
	request := httptest.NewRequest(http.MethodGet, "/me", nil)
	request.Header.Set("Authorization", "Bearer token")
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", response.Code, response.Body.String())
	}
	if repo.value.UserID != "user-123" {
		t.Fatalf("expected Cognito sub, got %q", repo.value.UserID)
	}
}
