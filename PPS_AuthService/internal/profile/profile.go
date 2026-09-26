package profile

import (
	"context"

	"github.com/porsche-performance-studio/pps-auth-service/internal/authmodel"
)

type Profile = authmodel.UserProfile

type Identity struct {
	UserID        string
	Issuer        string
	Email         string
	EmailVerified bool
}

type Update struct {
	DisplayName string `json:"displayName"`
	Locale      string `json:"locale"`
	Timezone    string `json:"timezone"`
}

type Repository interface {
	GetOrCreate(context.Context, Identity) (Profile, error)
	Update(context.Context, Identity, Update) (Profile, error)
}
