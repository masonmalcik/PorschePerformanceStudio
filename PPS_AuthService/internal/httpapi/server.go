package httpapi

import (
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"errors"
	"io"
	"log/slog"
	"net/http"
	"net/url"
	"regexp"
	"strings"
	"time"

	"github.com/porsche-performance-studio/pps-auth-service/internal/config"
	"github.com/porsche-performance-studio/pps-auth-service/internal/profile"
)

var verifierPattern = regexp.MustCompile(`^[A-Za-z0-9._~-]{43,128}$`)

type Server struct {
	cfg      config.Config
	profiles profile.Repository
	client   *http.Client
	logger   *slog.Logger
}

func New(cfg config.Config, profiles profile.Repository, client *http.Client, logger *slog.Logger) http.Handler {
	server := &Server{cfg: cfg, profiles: profiles, client: client, logger: logger}
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", server.health)
	mux.HandleFunc("GET /.well-known/openid-configuration", server.discovery)
	mux.HandleFunc("GET /.well-known/jwks.json", server.jwks)
	mux.HandleFunc("GET /oauth/authorize", server.authorize)
	mux.HandleFunc("POST /oauth/token", server.token)
	mux.HandleFunc("GET /oauth/logout", server.logout)
	mux.HandleFunc("GET /me", server.getMe)
	mux.HandleFunc("PUT /me", server.updateMe)
	return requestLog(logger, cors(cfg.FrontendOrigins, securityHeaders(mux)))
}

func (s *Server) health(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, http.StatusOK, map[string]string{"status": "healthy", "service": "pps-auth-service"})
}

func (s *Server) discovery(w http.ResponseWriter, r *http.Request) {
	proxyGET(w, r, s.client, s.cfg.Issuer()+"/.well-known/openid-configuration")
}

func (s *Server) jwks(w http.ResponseWriter, r *http.Request) {
	proxyGET(w, r, s.client, s.cfg.JWKSURL())
}

func (s *Server) authorize(w http.ResponseWriter, r *http.Request) {
	redirectURI := r.URL.Query().Get("redirect_uri")
	state := r.URL.Query().Get("state")
	challenge := r.URL.Query().Get("code_challenge")
	if !allowed(s.cfg.CallbackURLs, redirectURI) || len(state) < 16 || !verifierPattern.MatchString(challenge) {
		writeError(w, http.StatusBadRequest, "invalid_request", "redirect_uri, state, and an S256 PKCE code_challenge are required")
		return
	}
	values := url.Values{
		"response_type": {"code"}, "client_id": {s.cfg.ClientID}, "redirect_uri": {redirectURI},
		"scope": {strings.Join(s.cfg.Scopes, " ")}, "state": {state},
		"code_challenge": {challenge}, "code_challenge_method": {"S256"},
	}
	http.Redirect(w, r, s.cfg.CognitoDomain+"/oauth2/authorize?"+values.Encode(), http.StatusFound)
}

type tokenRequest struct {
	Code         string `json:"code"`
	CodeVerifier string `json:"codeVerifier"`
	RedirectURI  string `json:"redirectUri"`
}

func (s *Server) token(w http.ResponseWriter, r *http.Request) {
	var input tokenRequest
	if err := decodeJSON(r, &input); err != nil || input.Code == "" || !verifierPattern.MatchString(input.CodeVerifier) || !allowed(s.cfg.CallbackURLs, input.RedirectURI) {
		writeError(w, http.StatusBadRequest, "invalid_request", "code, valid codeVerifier, and approved redirectUri are required")
		return
	}
	form := url.Values{"grant_type": {"authorization_code"}, "client_id": {s.cfg.ClientID}, "code": {input.Code}, "code_verifier": {input.CodeVerifier}, "redirect_uri": {input.RedirectURI}}
	request, _ := http.NewRequestWithContext(r.Context(), http.MethodPost, s.cfg.CognitoDomain+"/oauth2/token", strings.NewReader(form.Encode()))
	request.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	response, err := s.client.Do(request)
	if err != nil {
		writeError(w, http.StatusBadGateway, "identity_provider_unavailable", "Cognito token endpoint is unavailable")
		return
	}
	defer response.Body.Close()
	copyResponse(w, response)
}

func (s *Server) logout(w http.ResponseWriter, r *http.Request) {
	logoutURI := r.URL.Query().Get("logout_uri")
	if !allowed(s.cfg.LogoutURLs, logoutURI) {
		writeError(w, http.StatusBadRequest, "invalid_request", "approved logout_uri is required")
		return
	}
	values := url.Values{"client_id": {s.cfg.ClientID}, "logout_uri": {logoutURI}}
	http.Redirect(w, r, s.cfg.CognitoDomain+"/logout?"+values.Encode(), http.StatusFound)
}

type cognitoUser struct {
	Sub           string `json:"sub"`
	Email         string `json:"email"`
	EmailVerified bool   `json:"email_verified"`
	Username      string `json:"username"`
}

func (s *Server) getMe(w http.ResponseWriter, r *http.Request) {
	user, err := s.userInfo(r)
	if err != nil {
		writeError(w, http.StatusUnauthorized, "unauthorized", "a valid Cognito access token is required")
		return
	}
	value, err := s.profiles.GetOrCreate(r.Context(), s.identity(user))
	if err != nil {
		s.internalError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"identity": user, "profile": value})
}

func (s *Server) updateMe(w http.ResponseWriter, r *http.Request) {
	user, err := s.userInfo(r)
	if err != nil {
		writeError(w, http.StatusUnauthorized, "unauthorized", "a valid Cognito access token is required")
		return
	}
	var update profile.Update
	if err := decodeJSON(r, &update); err != nil || len(update.DisplayName) > 100 || len(update.Locale) > 20 || len(update.Timezone) > 100 {
		writeError(w, http.StatusBadRequest, "invalid_request", "profile fields are invalid")
		return
	}
	value, err := s.profiles.Update(r.Context(), s.identity(user), update)
	if err != nil {
		s.internalError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, value)
}

func (s *Server) identity(user cognitoUser) profile.Identity {
	return profile.Identity{UserID: user.Sub, Issuer: s.cfg.Issuer(), Email: user.Email, EmailVerified: user.EmailVerified}
}

func (s *Server) userInfo(r *http.Request) (cognitoUser, error) {
	authorization := r.Header.Get("Authorization")
	if !strings.HasPrefix(authorization, "Bearer ") {
		return cognitoUser{}, errors.New("missing bearer token")
	}
	request, _ := http.NewRequestWithContext(r.Context(), http.MethodGet, s.cfg.CognitoDomain+"/oauth2/userInfo", nil)
	request.Header.Set("Authorization", authorization)
	response, err := s.client.Do(request)
	if err != nil {
		return cognitoUser{}, err
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return cognitoUser{}, errors.New("invalid token")
	}
	var user cognitoUser
	if err := json.NewDecoder(io.LimitReader(response.Body, 1<<20)).Decode(&user); err != nil || user.Sub == "" {
		return cognitoUser{}, errors.New("invalid userInfo response")
	}
	return user, nil
}

func (s *Server) internalError(w http.ResponseWriter, err error) {
	s.logger.Error("request failed", "error", err)
	writeError(w, http.StatusInternalServerError, "internal_error", "the request could not be completed")
}

func PKCEChallenge(verifier string) string {
	sum := sha256.Sum256([]byte(verifier))
	return base64.RawURLEncoding.EncodeToString(sum[:])
}
func allowed(values map[string]struct{}, value string) bool { _, ok := values[value]; return ok }

func decodeJSON(r *http.Request, target any) error {
	defer r.Body.Close()
	decoder := json.NewDecoder(io.LimitReader(r.Body, 1<<20))
	decoder.DisallowUnknownFields()
	return decoder.Decode(target)
}

func proxyGET(w http.ResponseWriter, r *http.Request, client *http.Client, target string) {
	request, _ := http.NewRequestWithContext(r.Context(), http.MethodGet, target, nil)
	response, err := client.Do(request)
	if err != nil {
		writeError(w, http.StatusBadGateway, "identity_provider_unavailable", "Cognito is unavailable")
		return
	}
	defer response.Body.Close()
	copyResponse(w, response)
}

func copyResponse(w http.ResponseWriter, response *http.Response) {
	w.Header().Set("Content-Type", response.Header.Get("Content-Type"))
	w.Header().Set("Cache-Control", response.Header.Get("Cache-Control"))
	w.WriteHeader(response.StatusCode)
	_, _ = io.Copy(w, io.LimitReader(response.Body, 2<<20))
}

func writeJSON(w http.ResponseWriter, status int, body any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(body)
}
func writeError(w http.ResponseWriter, status int, code, message string) {
	writeJSON(w, status, map[string]any{"error": map[string]string{"code": code, "message": message}})
}

func securityHeaders(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("X-Content-Type-Options", "nosniff")
		w.Header().Set("Referrer-Policy", "no-referrer")
		next.ServeHTTP(w, r)
	})
}

func cors(origins map[string]struct{}, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		origin := r.Header.Get("Origin")
		if allowed(origins, origin) {
			w.Header().Set("Access-Control-Allow-Origin", origin)
			w.Header().Set("Vary", "Origin")
			w.Header().Set("Access-Control-Allow-Headers", "Authorization, Content-Type, X-Request-ID")
			w.Header().Set("Access-Control-Allow-Methods", "GET, PUT, POST, OPTIONS")
		}
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
}

func requestLog(logger *slog.Logger, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		started := time.Now()
		next.ServeHTTP(w, r)
		logger.Info("http_request", "method", r.Method, "path", r.URL.Path, "duration_ms", time.Since(started).Milliseconds())
	})
}
