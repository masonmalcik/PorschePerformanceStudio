package authn

import (
	"context"
	"crypto/rsa"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math/big"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

var (
	ErrInvalidToken    = errors.New("invalid access token")
	ErrJWKSUnavailable = errors.New("JWKS unavailable")
)

type Verifier interface {
	Verify(context.Context, string) (Claims, error)
}

type Options struct {
	Issuer          string
	ClientID        string
	JWKSURL         string
	HTTPClient      *http.Client
	RefreshInterval time.Duration
	RetryBackoff    []time.Duration
}

type CachedVerifier struct {
	issuer          string
	clientID        string
	jwksURL         string
	client          *http.Client
	refreshInterval time.Duration
	retryBackoff    []time.Duration

	mu                sync.RWMutex
	keys              map[string]*rsa.PublicKey
	refreshAfter      time.Time
	lastForcedRefresh time.Time
	stop              chan struct{}
	stopOnce          sync.Once
}

type jwksDocument struct {
	Keys []jwk `json:"keys"`
}

type jwk struct {
	KID string `json:"kid"`
	KTY string `json:"kty"`
	Use string `json:"use"`
	Alg string `json:"alg"`
	N   string `json:"n"`
	E   string `json:"e"`
}

func NewCachedVerifier(options Options) (*CachedVerifier, error) {
	if options.Issuer == "" || options.ClientID == "" || options.JWKSURL == "" {
		return nil, errors.New("issuer, client ID, and JWKS URL are required")
	}
	if options.HTTPClient == nil {
		options.HTTPClient = &http.Client{Timeout: 5 * time.Second}
	}
	if options.RefreshInterval <= 0 {
		options.RefreshInterval = 24 * time.Hour
	}
	if len(options.RetryBackoff) == 0 {
		options.RetryBackoff = []time.Duration{0, 100 * time.Millisecond, 300 * time.Millisecond}
	}
	v := &CachedVerifier{
		issuer: options.Issuer, clientID: options.ClientID, jwksURL: options.JWKSURL,
		client: options.HTTPClient, refreshInterval: options.RefreshInterval,
		retryBackoff: append([]time.Duration(nil), options.RetryBackoff...),
		keys:         map[string]*rsa.PublicKey{}, stop: make(chan struct{}),
	}
	go v.refreshLoop()
	return v, nil
}

func (v *CachedVerifier) Close() { v.stopOnce.Do(func() { close(v.stop) }) }

func (v *CachedVerifier) Verify(ctx context.Context, rawToken string) (Claims, error) {
	if strings.TrimSpace(rawToken) == "" {
		return Claims{}, ErrInvalidToken
	}
	claims := Claims{}
	token, err := jwt.ParseWithClaims(rawToken, &claims, func(token *jwt.Token) (any, error) {
		if token.Method.Alg() != jwt.SigningMethodRS256.Alg() {
			return nil, fmt.Errorf("%w: signing algorithm %q is not allowed", ErrInvalidToken, token.Method.Alg())
		}
		kid, ok := token.Header["kid"].(string)
		if !ok || kid == "" {
			return nil, fmt.Errorf("%w: missing key ID", ErrInvalidToken)
		}
		return v.key(ctx, kid)
	}, jwt.WithIssuer(v.issuer), jwt.WithExpirationRequired(), jwt.WithValidMethods([]string{jwt.SigningMethodRS256.Alg()}))
	if err != nil {
		if errors.Is(err, ErrJWKSUnavailable) {
			return Claims{}, ErrJWKSUnavailable
		}
		return Claims{}, fmt.Errorf("%w: %v", ErrInvalidToken, err)
	}
	if !token.Valid || claims.Subject == "" || claims.TokenUse != "access" || claims.ClientID != v.clientID {
		return Claims{}, ErrInvalidToken
	}
	return claims, nil
}

func (v *CachedVerifier) key(ctx context.Context, kid string) (*rsa.PublicKey, error) {
	now := time.Now()
	v.mu.RLock()
	key := v.keys[kid]
	fresh := now.Before(v.refreshAfter)
	v.mu.RUnlock()
	if key != nil && fresh {
		return key, nil
	}

	v.mu.Lock()
	defer v.mu.Unlock()
	now = time.Now()
	if key = v.keys[kid]; key != nil && now.Before(v.refreshAfter) {
		return key, nil
	}
	// Refresh expired caches and unknown key IDs. A short forced-refresh guard
	// prevents attacker-controlled KIDs from turning every request into HTTP I/O.
	if key == nil && now.Before(v.refreshAfter) && now.Sub(v.lastForcedRefresh) < time.Minute {
		return nil, fmt.Errorf("%w: unknown key ID", ErrInvalidToken)
	}
	if key == nil {
		v.lastForcedRefresh = now
	}
	if err := v.refreshLocked(ctx); err != nil {
		return nil, err
	}
	if key = v.keys[kid]; key == nil {
		return nil, fmt.Errorf("%w: unknown key ID", ErrInvalidToken)
	}
	return key, nil
}

func (v *CachedVerifier) refreshLoop() {
	ticker := time.NewTicker(v.refreshInterval)
	defer ticker.Stop()
	for {
		select {
		case <-ticker.C:
			ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
			v.mu.Lock()
			_ = v.refreshLocked(ctx)
			v.mu.Unlock()
			cancel()
		case <-v.stop:
			return
		}
	}
}

func (v *CachedVerifier) refreshLocked(ctx context.Context) error {
	var lastErr error
	for _, delay := range v.retryBackoff {
		if delay > 0 {
			timer := time.NewTimer(delay)
			select {
			case <-timer.C:
			case <-ctx.Done():
				timer.Stop()
				return fmt.Errorf("%w: %v", ErrJWKSUnavailable, ctx.Err())
			}
		}
		keys, err := v.fetch(ctx)
		if err == nil {
			v.keys = keys
			v.refreshAfter = time.Now().Add(v.refreshInterval)
			return nil
		}
		lastErr = err
	}
	return fmt.Errorf("%w: %v", ErrJWKSUnavailable, lastErr)
}

func (v *CachedVerifier) fetch(ctx context.Context) (map[string]*rsa.PublicKey, error) {
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, v.jwksURL, nil)
	if err != nil {
		return nil, err
	}
	request.Header.Set("Accept", "application/json")
	response, err := v.client.Do(request)
	if err != nil {
		return nil, err
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		_, _ = io.Copy(io.Discard, io.LimitReader(response.Body, 4096))
		return nil, fmt.Errorf("JWKS endpoint returned %s", response.Status)
	}
	var document jwksDocument
	decoder := json.NewDecoder(io.LimitReader(response.Body, 1<<20))
	if err := decoder.Decode(&document); err != nil {
		return nil, fmt.Errorf("decode JWKS: %w", err)
	}
	keys := make(map[string]*rsa.PublicKey, len(document.Keys))
	for _, value := range document.Keys {
		if value.KID == "" || value.KTY != "RSA" || value.Use != "sig" || value.Alg != "RS256" {
			continue
		}
		key, err := rsaKey(value.N, value.E)
		if err != nil {
			return nil, fmt.Errorf("decode key %q: %w", value.KID, err)
		}
		keys[value.KID] = key
	}
	if len(keys) == 0 {
		return nil, errors.New("JWKS contains no usable RS256 signing keys")
	}
	return keys, nil
}

func rsaKey(modulus, exponent string) (*rsa.PublicKey, error) {
	n, err := base64.RawURLEncoding.DecodeString(modulus)
	if err != nil || len(n) == 0 {
		return nil, errors.New("invalid RSA modulus")
	}
	e, err := base64.RawURLEncoding.DecodeString(exponent)
	if err != nil || len(e) == 0 || len(e) > 4 {
		return nil, errors.New("invalid RSA exponent")
	}
	exponentValue := 0
	for _, b := range e {
		exponentValue = exponentValue<<8 | int(b)
	}
	if exponentValue < 3 {
		return nil, errors.New("invalid RSA exponent")
	}
	return &rsa.PublicKey{N: new(big.Int).SetBytes(n), E: exponentValue}, nil
}
