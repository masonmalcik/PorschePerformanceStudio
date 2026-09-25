package profile

import (
	"context"
	"time"
)

type Profile struct {
	UserID         string    `json:"userId" dynamodbav:"userId"`
	Email          string    `json:"email" dynamodbav:"email"`
	DisplayName    string    `json:"displayName" dynamodbav:"displayName"`
	Locale         string    `json:"locale" dynamodbav:"locale"`
	Timezone       string    `json:"timezone" dynamodbav:"timezone"`
	MarketingOptIn bool      `json:"marketingOptIn" dynamodbav:"marketingOptIn"`
	CreatedAt      time.Time `json:"createdAt" dynamodbav:"createdAt"`
	UpdatedAt      time.Time `json:"updatedAt" dynamodbav:"updatedAt"`
}

type Update struct {
	DisplayName    string `json:"displayName"`
	Locale         string `json:"locale"`
	Timezone       string `json:"timezone"`
	MarketingOptIn bool   `json:"marketingOptIn"`
}

type Repository interface {
	GetOrCreate(context.Context, string, string) (Profile, error)
	Update(context.Context, string, string, Update) (Profile, error)
}
