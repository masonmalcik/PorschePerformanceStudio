package httpapi

import (
	"context"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"github.com/porsche-performance-studio/pps-auth-service/internal/authmodel"
	"github.com/porsche-performance-studio/pps-auth-service/internal/authn"
	"github.com/porsche-performance-studio/pps-auth-service/internal/authz"
	"github.com/porsche-performance-studio/pps-auth-service/internal/config"
	"github.com/porsche-performance-studio/pps-auth-service/internal/profile"
)

type memoryProfiles struct{ value profile.Profile }

type memoryAuthorization struct{}

type staticVerifier struct{ claims authn.Claims }

func (s staticVerifier) Verify(context.Context, string) (authn.Claims, error) { return s.claims, nil }

func testVerifier() staticVerifier {
	return staticVerifier{claims: authn.Claims{RegisteredClaims: jwt.RegisteredClaims{Subject: "user-123", Issuer: "https://cognito-idp.us-east-1.amazonaws.com/pool"}, Email: "owner@example.com", EmailVerified: true, TokenUse: "access", ClientID: "client"}}
}

func (*memoryAuthorization) Permissions(context.Context, string) (map[string]struct{}, error) {
	return map[string]struct{}{}, nil
}
func (*memoryAuthorization) ListRoles(context.Context) ([]authmodel.Role, error) { return nil, nil }
func (*memoryAuthorization) GetRole(context.Context, string) (authmodel.Role, []string, error) {
	return authmodel.Role{}, nil, authz.ErrNotFound
}
func (*memoryAuthorization) CreateRole(context.Context, string, string, string, string) (authmodel.Role, error) {
	return authmodel.Role{}, nil
}
func (*memoryAuthorization) SetRolePermissions(context.Context, string, string, []string, string) error {
	return nil
}
func (*memoryAuthorization) AssignRole(context.Context, string, string, string, string, string) error {
	return nil
}
func (*memoryAuthorization) RemoveRole(context.Context, string, string, string, string, string) error {
	return nil
}
func (*memoryAuthorization) ListOrganizations(context.Context) ([]authmodel.Organization, error) {
	return nil, nil
}
func (*memoryAuthorization) CreateOrganization(context.Context, string, string, string) (authmodel.Organization, error) {
	return authmodel.Organization{}, nil
}
func (*memoryAuthorization) UpsertMembership(context.Context, string, string, string, string, string) (authmodel.Membership, error) {
	return authmodel.Membership{}, nil
}
func (*memoryAuthorization) ListConsents(context.Context, string) ([]authmodel.Consent, error) {
	return nil, nil
}
func (*memoryAuthorization) RecordConsent(context.Context, string, string, string, bool, string, string) (authmodel.Consent, error) {
	return authmodel.Consent{}, nil
}

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
	handler := New(testConfig("https://example.auth.us-east-1.amazoncognito.com"), &memoryProfiles{}, &memoryAuthorization{}, testVerifier(), http.DefaultClient, slog.New(slog.NewTextHandler(io.Discard, nil)))
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
	handler := New(testConfig("https://example.invalid"), &memoryProfiles{}, &memoryAuthorization{}, testVerifier(), http.DefaultClient, slog.New(slog.NewTextHandler(io.Discard, nil)))
	request := httptest.NewRequest(http.MethodGet, "/oauth/authorize?redirect_uri=https%3A%2F%2Fevil.example&state=1234567890abcdef&code_challenge="+strings.Repeat("a", 43), nil)
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusBadRequest {
		t.Fatalf("expected 400, got %d", response.Code)
	}
}

func TestMeUsesCognitoSubjectAsProfileKey(t *testing.T) {
	repo := &memoryProfiles{}
	handler := New(testConfig("https://example.invalid"), repo, &memoryAuthorization{}, testVerifier(), http.DefaultClient, slog.New(slog.NewTextHandler(io.Discard, nil)))
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
