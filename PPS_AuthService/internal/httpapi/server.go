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

	"github.com/porsche-performance-studio/pps-auth-service/internal/authn"
	"github.com/porsche-performance-studio/pps-auth-service/internal/authz"
	"github.com/porsche-performance-studio/pps-auth-service/internal/config"
	"github.com/porsche-performance-studio/pps-auth-service/internal/profile"
)

var verifierPattern = regexp.MustCompile(`^[A-Za-z0-9._~-]{43,128}$`)

type Server struct {
	cfg      config.Config
	profiles profile.Repository
	authz    authz.Repository
	client   *http.Client
	logger   *slog.Logger
}

func New(cfg config.Config, profiles profile.Repository, authorization authz.Repository, verifier authn.Verifier, client *http.Client, logger *slog.Logger) http.Handler {
	server := &Server{cfg: cfg, profiles: profiles, authz: authorization, client: client, logger: logger}
	authenticate := authn.NewMiddleware(verifier, logger).Authenticate
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", server.health)
	mux.HandleFunc("GET /.well-known/openid-configuration", server.discovery)
	mux.HandleFunc("GET /.well-known/jwks.json", server.jwks)
	mux.HandleFunc("GET /oauth/authorize", server.authorize)
	mux.HandleFunc("POST /oauth/token", server.token)
	mux.HandleFunc("GET /oauth/logout", server.logout)
	mux.Handle("GET /me", authenticate(http.HandlerFunc(server.getMe)))
	mux.Handle("PUT /me", authenticate(http.HandlerFunc(server.updateMe)))
	mux.Handle("GET /roles", authenticate(http.HandlerFunc(server.listRoles)))
	mux.Handle("POST /roles", authenticate(http.HandlerFunc(server.createRole)))
	mux.Handle("GET /roles/{roleId}", authenticate(http.HandlerFunc(server.getRole)))
	mux.Handle("PUT /roles/{roleId}/permissions", authenticate(http.HandlerFunc(server.setRolePermissions)))
	mux.Handle("PUT /users/{userId}/roles/{roleId}", authenticate(http.HandlerFunc(server.assignRole)))
	mux.Handle("DELETE /users/{userId}/roles/{roleId}", authenticate(http.HandlerFunc(server.removeRole)))
	mux.Handle("GET /organizations", authenticate(http.HandlerFunc(server.listOrganizations)))
	mux.Handle("POST /organizations", authenticate(http.HandlerFunc(server.createOrganization)))
	mux.Handle("PUT /organizations/{organizationId}/members/{userId}", authenticate(http.HandlerFunc(server.upsertMembership)))
	mux.Handle("GET /me/consents", authenticate(http.HandlerFunc(server.listConsents)))
	mux.Handle("POST /me/consents", authenticate(http.HandlerFunc(server.recordConsent)))
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
	user, ok := s.authenticated(w, r)
	if !ok {
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
	user, ok := s.authenticated(w, r)
	if !ok {
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

func (s *Server) listRoles(w http.ResponseWriter, r *http.Request) {
	if _, ok := s.requirePermission(w, r, "roles:read"); !ok {
		return
	}
	roles, err := s.authz.ListRoles(r.Context())
	if err != nil {
		s.internalError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"roles": roles})
}

func (s *Server) getRole(w http.ResponseWriter, r *http.Request) {
	if _, ok := s.requirePermission(w, r, "roles:read"); !ok {
		return
	}
	role, permissions, err := s.authz.GetRole(r.Context(), r.PathValue("roleId"))
	if errors.Is(err, authz.ErrNotFound) {
		writeError(w, http.StatusNotFound, "not_found", "role was not found")
		return
	}
	if err != nil {
		s.internalError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"role": role, "permissions": permissions})
}

type createRoleRequest struct {
	RoleID      string `json:"roleId"`
	Name        string `json:"name"`
	Description string `json:"description"`
}

func (s *Server) createRole(w http.ResponseWriter, r *http.Request) {
	user, ok := s.requirePermission(w, r, "roles:manage")
	if !ok {
		return
	}
	var input createRoleRequest
	if decodeJSON(r, &input) != nil || !authz.ValidID(input.RoleID) || input.Name == "" || len(input.Name) > 100 || len(input.Description) > 500 {
		writeError(w, http.StatusBadRequest, "invalid_request", "valid roleId, name, and description are required")
		return
	}
	role, err := s.authz.CreateRole(r.Context(), user.Sub, input.RoleID, input.Name, input.Description)
	if errors.Is(err, authz.ErrConflict) {
		writeError(w, http.StatusConflict, "conflict", "role already exists")
		return
	}
	if err != nil {
		s.internalError(w, err)
		return
	}
	writeJSON(w, http.StatusCreated, role)
}

type permissionsRequest struct {
	Permissions []string `json:"permissions"`
}

func (s *Server) setRolePermissions(w http.ResponseWriter, r *http.Request) {
	user, ok := s.requirePermission(w, r, "roles:manage")
	if !ok {
		return
	}
	var input permissionsRequest
	if decodeJSON(r, &input) != nil || len(input.Permissions) > 30 {
		writeError(w, http.StatusBadRequest, "invalid_request", "permissions must contain at most 30 values")
		return
	}
	for _, permission := range input.Permissions {
		if !authz.ValidID(permission) {
			writeError(w, http.StatusBadRequest, "invalid_request", "permission identifiers are invalid")
			return
		}
	}
	err := s.authz.SetRolePermissions(r.Context(), user.Sub, r.PathValue("roleId"), input.Permissions, requestID(r))
	if errors.Is(err, authz.ErrNotFound) {
		writeError(w, http.StatusNotFound, "not_found", "role was not found")
		return
	}
	if err != nil {
		s.internalError(w, err)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

type roleAssignmentRequest struct {
	OrganizationID string `json:"organizationId"`
}

func (s *Server) assignRole(w http.ResponseWriter, r *http.Request) {
	user, ok := s.requirePermission(w, r, "roles:assign")
	if !ok {
		return
	}
	var input roleAssignmentRequest
	if r.ContentLength > 0 && decodeJSON(r, &input) != nil {
		writeError(w, http.StatusBadRequest, "invalid_request", "request body is invalid")
		return
	}
	if err := s.authz.AssignRole(r.Context(), user.Sub, r.PathValue("userId"), r.PathValue("roleId"), input.OrganizationID, requestID(r)); err != nil {
		s.internalError(w, err)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

func (s *Server) removeRole(w http.ResponseWriter, r *http.Request) {
	user, ok := s.requirePermission(w, r, "roles:assign")
	if !ok {
		return
	}
	if err := s.authz.RemoveRole(r.Context(), user.Sub, r.PathValue("userId"), r.PathValue("roleId"), r.URL.Query().Get("organizationId"), requestID(r)); err != nil {
		s.internalError(w, err)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

func (s *Server) listOrganizations(w http.ResponseWriter, r *http.Request) {
	if _, ok := s.requirePermission(w, r, "organizations:read"); !ok {
		return
	}
	values, err := s.authz.ListOrganizations(r.Context())
	if err != nil {
		s.internalError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"organizations": values})
}

type createOrganizationRequest struct {
	Name string `json:"name"`
}

func (s *Server) createOrganization(w http.ResponseWriter, r *http.Request) {
	user, ok := s.requirePermission(w, r, "organizations:manage")
	if !ok {
		return
	}
	var input createOrganizationRequest
	if decodeJSON(r, &input) != nil || strings.TrimSpace(input.Name) == "" || len(input.Name) > 200 {
		writeError(w, http.StatusBadRequest, "invalid_request", "organization name is required")
		return
	}
	value, err := s.authz.CreateOrganization(r.Context(), user.Sub, input.Name, requestID(r))
	if err != nil {
		s.internalError(w, err)
		return
	}
	writeJSON(w, http.StatusCreated, value)
}

type membershipRequest struct {
	Status string `json:"status"`
}

func (s *Server) upsertMembership(w http.ResponseWriter, r *http.Request) {
	user, ok := s.requirePermission(w, r, "organizations:members:manage")
	if !ok {
		return
	}
	var input membershipRequest
	if decodeJSON(r, &input) != nil || (input.Status != "active" && input.Status != "suspended") {
		writeError(w, http.StatusBadRequest, "invalid_request", "status must be active or suspended")
		return
	}
	value, err := s.authz.UpsertMembership(r.Context(), user.Sub, r.PathValue("organizationId"), r.PathValue("userId"), input.Status, requestID(r))
	if err != nil {
		s.internalError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, value)
}

func (s *Server) listConsents(w http.ResponseWriter, r *http.Request) {
	user, ok := s.authenticated(w, r)
	if !ok {
		return
	}
	values, err := s.authz.ListConsents(r.Context(), user.Sub)
	if err != nil {
		s.internalError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"consents": values})
}

type consentRequest struct {
	ConsentType   string `json:"consentType"`
	PolicyVersion string `json:"policyVersion"`
	Granted       bool   `json:"granted"`
}

func (s *Server) recordConsent(w http.ResponseWriter, r *http.Request) {
	user, ok := s.authenticated(w, r)
	if !ok {
		return
	}
	var input consentRequest
	if decodeJSON(r, &input) != nil || !authz.ValidID(input.ConsentType) || input.PolicyVersion == "" || len(input.PolicyVersion) > 50 {
		writeError(w, http.StatusBadRequest, "invalid_request", "valid consentType and policyVersion are required")
		return
	}
	value, err := s.authz.RecordConsent(r.Context(), user.Sub, input.ConsentType, input.PolicyVersion, input.Granted, "api", requestID(r))
	if err != nil {
		s.internalError(w, err)
		return
	}
	writeJSON(w, http.StatusCreated, value)
}

func (s *Server) authenticated(w http.ResponseWriter, r *http.Request) (cognitoUser, bool) {
	claims, ok := authn.ClaimsFromContext(r.Context())
	if !ok {
		writeError(w, http.StatusUnauthorized, "unauthorized", "a valid Cognito access token is required")
		return cognitoUser{}, false
	}
	user := cognitoUser{Sub: claims.Subject, Email: claims.Email, EmailVerified: claims.EmailVerified, Username: claims.EffectiveUsername()}
	if _, err := s.profiles.GetOrCreate(r.Context(), s.identity(user)); err != nil {
		s.internalError(w, err)
		return cognitoUser{}, false
	}
	return user, true
}

func (s *Server) requirePermission(w http.ResponseWriter, r *http.Request, permission string) (cognitoUser, bool) {
	user, ok := s.authenticated(w, r)
	if !ok {
		return cognitoUser{}, false
	}
	permissions, err := s.authz.Permissions(r.Context(), user.Sub)
	if err != nil {
		s.internalError(w, err)
		return cognitoUser{}, false
	}
	if _, allowed := permissions[permission]; !allowed {
		writeError(w, http.StatusForbidden, "forbidden", "the user does not have the required permission")
		return cognitoUser{}, false
	}
	return user, true
}

func requestID(r *http.Request) string { return r.Header.Get("X-Request-ID") }

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
			w.Header().Set("Access-Control-Allow-Methods", "GET, PUT, POST, DELETE, OPTIONS")
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
