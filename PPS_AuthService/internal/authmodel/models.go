package authmodel

import "time"

const (
	EntityUserProfile    = "USER_PROFILE"
	EntityRole           = "ROLE"
	EntityPermission     = "PERMISSION"
	EntityUserRole       = "USER_ROLE"
	EntityRolePermission = "ROLE_PERMISSION"
	EntityOrganization   = "ORGANIZATION"
	EntityMembership     = "MEMBERSHIP"
	EntityIdentityLink   = "IDENTITY_LINK"
	EntityConsent        = "CONSENT"
	EntityAuditEvent     = "AUDIT_EVENT"
	EntityMigration      = "MIGRATION"
)

type UserStatus string

const (
	UserStatusActive    UserStatus = "active"
	UserStatusSuspended UserStatus = "suspended"
	UserStatusDisabled  UserStatus = "disabled"
)

type UserProfile struct {
	PK            string     `json:"-" dynamodbav:"PK"`
	SK            string     `json:"-" dynamodbav:"SK"`
	EntityType    string     `json:"-" dynamodbav:"entityType"`
	UserID        string     `json:"userId" dynamodbav:"userId"`
	Issuer        string     `json:"issuer" dynamodbav:"issuer"`
	Email         string     `json:"email" dynamodbav:"email"`
	EmailVerified bool       `json:"emailVerified" dynamodbav:"emailVerified"`
	DisplayName   string     `json:"displayName" dynamodbav:"displayName"`
	Locale        string     `json:"locale" dynamodbav:"locale"`
	Timezone      string     `json:"timezone" dynamodbav:"timezone"`
	Status        UserStatus `json:"status" dynamodbav:"status"`
	Version       int64      `json:"version" dynamodbav:"version"`
	CreatedAt     time.Time  `json:"createdAt" dynamodbav:"createdAt"`
	UpdatedAt     time.Time  `json:"updatedAt" dynamodbav:"updatedAt"`
}

type Role struct {
	PK          string    `json:"-" dynamodbav:"PK"`
	SK          string    `json:"-" dynamodbav:"SK"`
	EntityType  string    `json:"-" dynamodbav:"entityType"`
	RoleID      string    `json:"roleId" dynamodbav:"roleId"`
	Name        string    `json:"name" dynamodbav:"name"`
	Description string    `json:"description" dynamodbav:"description"`
	System      bool      `json:"system" dynamodbav:"system"`
	CreatedAt   time.Time `json:"createdAt" dynamodbav:"createdAt"`
	UpdatedAt   time.Time `json:"updatedAt" dynamodbav:"updatedAt"`
}

type Permission struct {
	PK          string `json:"-" dynamodbav:"PK"`
	SK          string `json:"-" dynamodbav:"SK"`
	EntityType  string `json:"-" dynamodbav:"entityType"`
	Code        string `json:"code" dynamodbav:"code"`
	Description string `json:"description" dynamodbav:"description"`
}

type UserRole struct {
	PK             string     `json:"-" dynamodbav:"PK"`
	SK             string     `json:"-" dynamodbav:"SK"`
	GSI1PK         string     `json:"-" dynamodbav:"GSI1PK"`
	GSI1SK         string     `json:"-" dynamodbav:"GSI1SK"`
	EntityType     string     `json:"-" dynamodbav:"entityType"`
	UserID         string     `json:"userId" dynamodbav:"userId"`
	RoleID         string     `json:"roleId" dynamodbav:"roleId"`
	OrganizationID string     `json:"organizationId,omitempty" dynamodbav:"organizationId,omitempty"`
	GrantedBy      string     `json:"grantedBy" dynamodbav:"grantedBy"`
	GrantedAt      time.Time  `json:"grantedAt" dynamodbav:"grantedAt"`
	ExpiresAt      *time.Time `json:"expiresAt,omitempty" dynamodbav:"expiresAt,omitempty"`
}

type RolePermission struct {
	PK           string    `json:"-" dynamodbav:"PK"`
	SK           string    `json:"-" dynamodbav:"SK"`
	EntityType   string    `json:"-" dynamodbav:"entityType"`
	RoleID       string    `json:"roleId" dynamodbav:"roleId"`
	PermissionID string    `json:"permissionId" dynamodbav:"permissionId"`
	GrantedAt    time.Time `json:"grantedAt" dynamodbav:"grantedAt"`
}

type Organization struct {
	PK             string    `json:"-" dynamodbav:"PK"`
	SK             string    `json:"-" dynamodbav:"SK"`
	EntityType     string    `json:"-" dynamodbav:"entityType"`
	OrganizationID string    `json:"organizationId" dynamodbav:"organizationId"`
	Name           string    `json:"name" dynamodbav:"name"`
	Status         string    `json:"status" dynamodbav:"status"`
	CreatedAt      time.Time `json:"createdAt" dynamodbav:"createdAt"`
	UpdatedAt      time.Time `json:"updatedAt" dynamodbav:"updatedAt"`
}

type Membership struct {
	PK             string    `json:"-" dynamodbav:"PK"`
	SK             string    `json:"-" dynamodbav:"SK"`
	GSI1PK         string    `json:"-" dynamodbav:"GSI1PK"`
	GSI1SK         string    `json:"-" dynamodbav:"GSI1SK"`
	EntityType     string    `json:"-" dynamodbav:"entityType"`
	OrganizationID string    `json:"organizationId" dynamodbav:"organizationId"`
	UserID         string    `json:"userId" dynamodbav:"userId"`
	Status         string    `json:"status" dynamodbav:"status"`
	JoinedAt       time.Time `json:"joinedAt" dynamodbav:"joinedAt"`
}

type IdentityLink struct {
	PK         string    `json:"-" dynamodbav:"PK"`
	SK         string    `json:"-" dynamodbav:"SK"`
	EntityType string    `json:"-" dynamodbav:"entityType"`
	Issuer     string    `json:"issuer" dynamodbav:"issuer"`
	Subject    string    `json:"subject" dynamodbav:"subject"`
	UserID     string    `json:"userId" dynamodbav:"userId"`
	Provider   string    `json:"provider" dynamodbav:"provider"`
	LinkedAt   time.Time `json:"linkedAt" dynamodbav:"linkedAt"`
}

type Consent struct {
	PK            string     `json:"-" dynamodbav:"PK"`
	SK            string     `json:"-" dynamodbav:"SK"`
	EntityType    string     `json:"-" dynamodbav:"entityType"`
	UserID        string     `json:"userId" dynamodbav:"userId"`
	ConsentType   string     `json:"consentType" dynamodbav:"consentType"`
	PolicyVersion string     `json:"policyVersion" dynamodbav:"policyVersion"`
	Granted       bool       `json:"granted" dynamodbav:"granted"`
	RecordedAt    time.Time  `json:"recordedAt" dynamodbav:"recordedAt"`
	Source        string     `json:"source" dynamodbav:"source"`
	RevokedAt     *time.Time `json:"revokedAt,omitempty" dynamodbav:"revokedAt,omitempty"`
}

type AuditEvent struct {
	PK           string         `json:"-" dynamodbav:"PK"`
	SK           string         `json:"-" dynamodbav:"SK"`
	EntityType   string         `json:"-" dynamodbav:"entityType"`
	EventID      string         `json:"eventId" dynamodbav:"eventId"`
	ActorUserID  string         `json:"actorUserId" dynamodbav:"actorUserId"`
	Action       string         `json:"action" dynamodbav:"action"`
	ResourceType string         `json:"resourceType" dynamodbav:"resourceType"`
	ResourceID   string         `json:"resourceId" dynamodbav:"resourceId"`
	Timestamp    time.Time      `json:"timestamp" dynamodbav:"timestamp"`
	RequestID    string         `json:"requestId" dynamodbav:"requestId"`
	Metadata     map[string]any `json:"metadata,omitempty" dynamodbav:"metadata,omitempty"`
}

type MigrationRecord struct {
	PK          string    `json:"-" dynamodbav:"PK"`
	SK          string    `json:"-" dynamodbav:"SK"`
	EntityType  string    `json:"-" dynamodbav:"entityType"`
	MigrationID string    `json:"migrationId" dynamodbav:"migrationId"`
	Checksum    string    `json:"checksum" dynamodbav:"checksum"`
	Status      string    `json:"status" dynamodbav:"status"`
	AppliedAt   time.Time `json:"appliedAt" dynamodbav:"appliedAt"`
	DurationMS  int64     `json:"durationMs" dynamodbav:"durationMs"`
}
