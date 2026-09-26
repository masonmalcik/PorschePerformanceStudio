package profile

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/feature/dynamodb/attributevalue"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb/types"
	"github.com/porsche-performance-studio/pps-auth-service/internal/authmodel"
)

var ErrConflict = errors.New("profile was modified concurrently")

type DynamoDBAPI interface {
	GetItem(context.Context, *dynamodb.GetItemInput, ...func(*dynamodb.Options)) (*dynamodb.GetItemOutput, error)
	PutItem(context.Context, *dynamodb.PutItemInput, ...func(*dynamodb.Options)) (*dynamodb.PutItemOutput, error)
	TransactWriteItems(context.Context, *dynamodb.TransactWriteItemsInput, ...func(*dynamodb.Options)) (*dynamodb.TransactWriteItemsOutput, error)
}

type DynamoRepository struct {
	client DynamoDBAPI
	table  string
	now    func() time.Time
}

func NewDynamoRepository(client DynamoDBAPI, table string) *DynamoRepository {
	return &DynamoRepository{client: client, table: table, now: func() time.Time { return time.Now().UTC() }}
}

func (r *DynamoRepository) GetOrCreate(ctx context.Context, identity Identity) (Profile, error) {
	current, found, err := r.get(ctx, identity.UserID)
	if err != nil {
		return Profile{}, err
	}
	if found {
		if current.Email != identity.Email || current.EmailVerified != identity.EmailVerified || current.Issuer != identity.Issuer {
			current.Email = identity.Email
			current.EmailVerified = identity.EmailVerified
			current.Issuer = identity.Issuer
			return r.save(ctx, current)
		}
		return current, nil
	}

	now := r.now()
	current = Profile{
		PK: "USER#" + identity.UserID, SK: "PROFILE", EntityType: authmodel.EntityUserProfile,
		UserID: identity.UserID, Issuer: identity.Issuer, Email: identity.Email, EmailVerified: identity.EmailVerified,
		Status: authmodel.UserStatusActive, Version: 1, CreatedAt: now, UpdatedAt: now,
	}
	profileItem, err := attributevalue.MarshalMap(current)
	if err != nil {
		return Profile{}, err
	}
	defaultRole := authmodel.UserRole{
		PK: current.PK, SK: "ROLE#customer", GSI1PK: "ROLE#customer", GSI1SK: current.PK,
		EntityType: authmodel.EntityUserRole, UserID: identity.UserID, RoleID: "customer",
		GrantedBy: "system", GrantedAt: now,
	}
	roleItem, err := attributevalue.MarshalMap(defaultRole)
	if err != nil {
		return Profile{}, err
	}
	_, err = r.client.TransactWriteItems(ctx, &dynamodb.TransactWriteItemsInput{TransactItems: []types.TransactWriteItem{
		{Put: &types.Put{TableName: aws.String(r.table), Item: profileItem, ConditionExpression: aws.String("attribute_not_exists(PK)")}},
		{Put: &types.Put{TableName: aws.String(r.table), Item: roleItem, ConditionExpression: aws.String("attribute_not_exists(PK)")}},
	}})
	if err != nil {
		if existing, exists, getErr := r.get(ctx, identity.UserID); getErr == nil && exists {
			return existing, nil
		}
		return Profile{}, fmt.Errorf("create profile and default role: %w", err)
	}
	return current, nil
}

func (r *DynamoRepository) Update(ctx context.Context, identity Identity, update Update) (Profile, error) {
	current, err := r.GetOrCreate(ctx, identity)
	if err != nil {
		return Profile{}, err
	}
	current.DisplayName = update.DisplayName
	current.Locale = update.Locale
	current.Timezone = update.Timezone
	return r.save(ctx, current)
}

func (r *DynamoRepository) save(ctx context.Context, current Profile) (Profile, error) {
	expectedVersion := current.Version
	current.Version++
	current.UpdatedAt = r.now()
	item, err := attributevalue.MarshalMap(current)
	if err != nil {
		return Profile{}, err
	}
	version, err := attributevalue.Marshal(expectedVersion)
	if err != nil {
		return Profile{}, err
	}
	_, err = r.client.PutItem(ctx, &dynamodb.PutItemInput{
		TableName: aws.String(r.table), Item: item,
		ConditionExpression:       aws.String("#version = :expectedVersion"),
		ExpressionAttributeNames:  map[string]string{"#version": "version"},
		ExpressionAttributeValues: map[string]types.AttributeValue{":expectedVersion": version},
	})
	if err != nil {
		var conditional *types.ConditionalCheckFailedException
		if errors.As(err, &conditional) {
			return Profile{}, ErrConflict
		}
		return Profile{}, err
	}
	return current, nil
}

func (r *DynamoRepository) get(ctx context.Context, userID string) (Profile, bool, error) {
	output, err := r.client.GetItem(ctx, &dynamodb.GetItemInput{
		TableName: aws.String(r.table), ConsistentRead: aws.Bool(true),
		Key: map[string]types.AttributeValue{
			"PK": &types.AttributeValueMemberS{Value: "USER#" + userID},
			"SK": &types.AttributeValueMemberS{Value: "PROFILE"},
		},
	})
	if err != nil {
		return Profile{}, false, err
	}
	if len(output.Item) == 0 {
		return Profile{}, false, nil
	}
	var current Profile
	if err := attributevalue.UnmarshalMap(output.Item, &current); err != nil {
		return Profile{}, false, err
	}
	return current, true, nil
}
