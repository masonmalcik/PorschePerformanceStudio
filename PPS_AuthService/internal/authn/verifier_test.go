package authn

import (
	"context"
	"crypto/rand"
	"crypto/rsa"
	"encoding/base64"
	"encoding/json"
	"errors"
	"io"
	"log/slog"
	"math/big"
	"net/http"
	"net/http/httptest"
	"sync/atomic"
	"testing"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

func TestCachedVerifierFetchesJWKSOnceAndParsesClaims(t *testing.T) {
	privateKey := testRSAKey(t)
	var requests atomic.Int32
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		requests.Add(1)
		writeTestJWKS(t, w, "key-1", &privateKey.PublicKey)
	}))
	defer server.Close()

	verifier, err := NewCachedVerifier(Options{
		Issuer: "https://issuer.example/pool", ClientID: "client-123", JWKSURL: server.URL,
		HTTPClient: server.Client(), RefreshInterval: 24 * time.Hour, RetryBackoff: []time.Duration{0},
	})
	if err != nil {
		t.Fatal(err)
	}
	defer verifier.Close()
	token := signTestToken(t, privateKey, "key-1", "client-123")

	for range 2 {
		claims, err := verifier.Verify(context.Background(), token)
		if err != nil {
			t.Fatal(err)
		}
		if claims.Subject != "user-123" || claims.EffectiveUsername() != "owner" || len(claims.Groups) != 2 {
			t.Fatalf("unexpected claims: %+v", claims)
		}
	}
	if requests.Load() != 1 {
		t.Fatalf("expected one JWKS fetch, got %d", requests.Load())
	}
}

func TestCachedVerifierRetriesAndReturnsUnavailable(t *testing.T) {
	var requests atomic.Int32
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		requests.Add(1)
		http.Error(w, "temporary failure", http.StatusServiceUnavailable)
	}))
	defer server.Close()
	verifier, err := NewCachedVerifier(Options{
		Issuer: "https://issuer.example/pool", ClientID: "client-123", JWKSURL: server.URL,
		HTTPClient: server.Client(), RetryBackoff: []time.Duration{0, time.Millisecond, time.Millisecond},
	})
	if err != nil {
		t.Fatal(err)
	}
	defer verifier.Close()
	_, err = verifier.Verify(context.Background(), "eyJhbGciOiJSUzI1NiIsImtpZCI6ImtleS0xIn0.eyJleHAiOjQxMDI0NDQ4MDB9.c2ln")
	if !errors.Is(err, ErrJWKSUnavailable) {
		t.Fatalf("expected JWKS unavailable, got %v", err)
	}
	if requests.Load() != 3 {
		t.Fatalf("expected three attempts, got %d", requests.Load())
	}
}

func TestMiddlewareReturns503WhenJWKSUnavailable(t *testing.T) {
	middleware := NewMiddleware(errorVerifier{err: ErrJWKSUnavailable}, slog.New(slog.NewTextHandler(io.Discard, nil)))
	handler := middleware.Authenticate(http.HandlerFunc(func(http.ResponseWriter, *http.Request) {
		t.Fatal("protected handler must not run")
	}))
	request := httptest.NewRequest(http.MethodGet, "/me", nil)
	request.Header.Set("Authorization", "Bearer token")
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusServiceUnavailable || response.Header().Get("Retry-After") != "1" {
		t.Fatalf("expected retryable 503, got %d", response.Code)
	}
}

func TestMiddlewareAttachesClaimsToContext(t *testing.T) {
	expected := Claims{RegisteredClaims: jwt.RegisteredClaims{Subject: "user-123"}, Groups: []string{"admin"}}
	middleware := NewMiddleware(errorVerifier{claims: expected}, slog.New(slog.NewTextHandler(io.Discard, nil)))
	handler := middleware.Authenticate(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		claims, ok := ClaimsFromContext(r.Context())
		if !ok || claims.Subject != expected.Subject || len(claims.Groups) != 1 {
			t.Fatalf("verified claims missing from context: %+v", claims)
		}
		w.WriteHeader(http.StatusNoContent)
	}))
	request := httptest.NewRequest(http.MethodGet, "/me", nil)
	request.Header.Set("Authorization", "Bearer token")
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusNoContent {
		t.Fatalf("expected 204, got %d", response.Code)
	}
}

type errorVerifier struct {
	claims Claims
	err    error
}

func (v errorVerifier) Verify(context.Context, string) (Claims, error) { return v.claims, v.err }

func testRSAKey(t *testing.T) *rsa.PrivateKey {
	t.Helper()
	key, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		t.Fatal(err)
	}
	return key
}

func writeTestJWKS(t *testing.T, w http.ResponseWriter, kid string, key *rsa.PublicKey) {
	t.Helper()
	exponent := big.NewInt(int64(key.E)).Bytes()
	w.Header().Set("Content-Type", "application/json")
	if err := json.NewEncoder(w).Encode(jwksDocument{Keys: []jwk{{
		KID: kid, KTY: "RSA", Use: "sig", Alg: "RS256",
		N: base64.RawURLEncoding.EncodeToString(key.N.Bytes()), E: base64.RawURLEncoding.EncodeToString(exponent),
	}}}); err != nil {
		t.Error(err)
	}
}

func signTestToken(t *testing.T, key *rsa.PrivateKey, kid, clientID string) string {
	t.Helper()
	claims := Claims{
		RegisteredClaims: jwt.RegisteredClaims{
			Issuer: "https://issuer.example/pool", Subject: "user-123",
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(time.Hour)), IssuedAt: jwt.NewNumericDate(time.Now()),
		},
		CognitoUser: "owner", Groups: []string{"admin", "customer"}, Email: "owner@example.com",
		EmailVerified: true, TokenUse: "access", ClientID: clientID,
	}
	token := jwt.NewWithClaims(jwt.SigningMethodRS256, claims)
	token.Header["kid"] = kid
	raw, err := token.SignedString(key)
	if err != nil {
		t.Fatal(err)
	}
	return raw
}
