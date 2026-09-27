package authn

import (
	"context"
	"crypto/rand"
	"crypto/rsa"
	"encoding/base64"
	"encoding/json"
	"io"
	"log/slog"
	"math/big"
	"net/http"
	"net/http/httptest"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

const (
	testKeyID    = "cognito-test-key"
	testClientID = "pps-test-client"
	testSubject  = "user-123"
)

func TestMainMiddlewareValidToken(t *testing.T) {
	privateKey := generateRSAKey(t)
	server, requests := newMockCognitoServer(t, testKeyID, &privateKey.PublicKey)
	defer server.Close()
	verifier := newTestCachedVerifier(t, server, testClientID)
	defer verifier.Close()

	handler := testAuthenticationHandler(verifier, func(w http.ResponseWriter, r *http.Request) {
		claims, ok := ClaimsFromContext(r.Context())
		if !ok || claims.Subject != testSubject || claims.EffectiveUsername() != "owner" {
			http.Error(w, "verified claims missing", http.StatusInternalServerError)
			return
		}
		if len(claims.Groups) != 2 || claims.Groups[0] != "administrator" {
			http.Error(w, "Cognito groups missing", http.StatusInternalServerError)
			return
		}
		w.WriteHeader(http.StatusOK)
	})
	token := signCognitoToken(t, privateKey, testKeyID, server.URL, testClientID, time.Now().Add(time.Hour))

	response := performAuthenticatedRequest(handler, token)

	if response.Code != http.StatusOK {
		t.Fatalf("expected valid token to return 200, got %d: %s", response.Code, response.Body.String())
	}
	if requests.Load() != 1 {
		t.Fatalf("expected one JWKS request, got %d", requests.Load())
	}
}

func TestMainMiddlewareExpiredToken(t *testing.T) {
	privateKey := generateRSAKey(t)
	server, _ := newMockCognitoServer(t, testKeyID, &privateKey.PublicKey)
	defer server.Close()
	verifier := newTestCachedVerifier(t, server, testClientID)
	defer verifier.Close()
	handler := testAuthenticationHandler(verifier, func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusOK)
	})
	token := signCognitoToken(t, privateKey, testKeyID, server.URL, testClientID, time.Now().Add(-time.Minute))

	response := performAuthenticatedRequest(handler, token)

	if response.Code != http.StatusUnauthorized {
		t.Fatalf("expected expired token to return 401, got %d: %s", response.Code, response.Body.String())
	}
}

func TestMainMiddlewareRejectsWrongSigningKey(t *testing.T) {
	trustedKey := generateRSAKey(t)
	maliciousKey := generateRSAKey(t)
	server, _ := newMockCognitoServer(t, testKeyID, &trustedKey.PublicKey)
	defer server.Close()
	verifier := newTestCachedVerifier(t, server, testClientID)
	defer verifier.Close()
	handler := testAuthenticationHandler(verifier, func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusOK)
	})
	token := signCognitoToken(t, maliciousKey, testKeyID, server.URL, testClientID, time.Now().Add(time.Hour))

	response := performAuthenticatedRequest(handler, token)

	if response.Code != http.StatusUnauthorized {
		t.Fatalf("expected wrong signing key to return 401, got %d: %s", response.Code, response.Body.String())
	}
}

func TestMainMiddlewareConcurrentJWKSCache(t *testing.T) {
	privateKey := generateRSAKey(t)
	server, jwksRequests := newMockCognitoServer(t, testKeyID, &privateKey.PublicKey)
	defer server.Close()
	verifier := newTestCachedVerifier(t, server, testClientID)
	defer verifier.Close()
	var handlerFailures atomic.Int64
	handler := testAuthenticationHandler(verifier, func(w http.ResponseWriter, r *http.Request) {
		claims, ok := ClaimsFromContext(r.Context())
		if !ok || claims.Subject != testSubject {
			handlerFailures.Add(1)
			http.Error(w, "verified claims missing", http.StatusInternalServerError)
			return
		}
		w.WriteHeader(http.StatusOK)
	})
	token := signCognitoToken(t, privateKey, testKeyID, server.URL, testClientID, time.Now().Add(time.Hour))

	const requestCount = 500
	start := make(chan struct{})
	statusCodes := make(chan int, requestCount)
	var workers sync.WaitGroup
	workers.Add(requestCount)
	for range requestCount {
		go func() {
			defer workers.Done()
			<-start
			statusCodes <- performAuthenticatedRequest(handler, token).Code
		}()
	}
	close(start)
	workers.Wait()
	close(statusCodes)

	failedResponses := 0
	for status := range statusCodes {
		if status != http.StatusOK {
			failedResponses++
		}
	}
	if failedResponses != 0 {
		t.Fatalf("expected all %d requests to return 200, got %d failures", requestCount, failedResponses)
	}
	if handlerFailures.Load() != 0 {
		t.Fatalf("expected verified claims in every request, got %d failures", handlerFailures.Load())
	}
	if jwksRequests.Load() != 1 {
		t.Fatalf("expected concurrent requests to share one JWKS fetch, got %d", jwksRequests.Load())
	}
}

func newMockCognitoServer(t *testing.T, keyID string, publicKey *rsa.PublicKey) (*httptest.Server, *atomic.Int64) {
	t.Helper()
	requests := &atomic.Int64{}
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet || r.URL.Path != "/.well-known/jwks.json" {
			http.NotFound(w, r)
			return
		}
		requests.Add(1)
		exponent := big.NewInt(int64(publicKey.E)).Bytes()
		w.Header().Set("Content-Type", "application/json")
		if err := json.NewEncoder(w).Encode(jwksDocument{Keys: []jwk{{
			KID: keyID,
			KTY: "RSA",
			Use: "sig",
			Alg: "RS256",
			N:   base64.RawURLEncoding.EncodeToString(publicKey.N.Bytes()),
			E:   base64.RawURLEncoding.EncodeToString(exponent),
		}}}); err != nil {
			http.Error(w, "could not encode JWKS", http.StatusInternalServerError)
		}
	}))
	return server, requests
}

func newTestCachedVerifier(t *testing.T, server *httptest.Server, clientID string) *CachedVerifier {
	t.Helper()
	verifier, err := NewCachedVerifier(Options{
		Issuer:          server.URL,
		ClientID:        clientID,
		JWKSURL:         server.URL + "/.well-known/jwks.json",
		HTTPClient:      server.Client(),
		RefreshInterval: 24 * time.Hour,
		RetryBackoff:    []time.Duration{0},
	})
	if err != nil {
		t.Fatalf("create cached verifier: %v", err)
	}
	return verifier
}

func testAuthenticationHandler(verifier Verifier, next http.HandlerFunc) http.Handler {
	logger := slog.New(slog.NewTextHandler(io.Discard, nil))
	return NewMiddleware(verifier, logger).Authenticate(next)
}

func performAuthenticatedRequest(handler http.Handler, token string) *httptest.ResponseRecorder {
	request := httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/protected", nil)
	request.Header.Set("Authorization", "Bearer "+token)
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	return response
}

func generateRSAKey(t *testing.T) *rsa.PrivateKey {
	t.Helper()
	key, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		t.Fatalf("generate RSA key: %v", err)
	}
	return key
}

func signCognitoToken(
	t *testing.T,
	privateKey *rsa.PrivateKey,
	keyID string,
	issuer string,
	clientID string,
	expiresAt time.Time,
) string {
	t.Helper()
	now := time.Now()
	claims := Claims{
		RegisteredClaims: jwt.RegisteredClaims{
			Issuer:    issuer,
			Subject:   testSubject,
			ExpiresAt: jwt.NewNumericDate(expiresAt),
			IssuedAt:  jwt.NewNumericDate(now.Add(-time.Second)),
		},
		CognitoUser:   "owner",
		Groups:        []string{"administrator", "customer"},
		Email:         "owner@example.com",
		EmailVerified: true,
		TokenUse:      "access",
		ClientID:      clientID,
		Scope:         "openid pps-api/profile.read",
	}
	token := jwt.NewWithClaims(jwt.SigningMethodRS256, claims)
	token.Header["kid"] = keyID
	raw, err := token.SignedString(privateKey)
	if err != nil {
		t.Fatalf("sign JWT: %v", err)
	}
	return raw
}
