package profile

import (
	"context"
	"errors"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/feature/dynamodb/attributevalue"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb/types"
)

type DynamoDBAPI interface {
	GetItem(context.Context, *dynamodb.GetItemInput, ...func(*dynamodb.Options)) (*dynamodb.GetItemOutput, error)
	PutItem(context.Context, *dynamodb.PutItemInput, ...func(*dynamodb.Options)) (*dynamodb.PutItemOutput, error)
}

type DynamoRepository struct {
	client DynamoDBAPI
	table  string
	now    func() time.Time
}

func NewDynamoRepository(client DynamoDBAPI, table string) *DynamoRepository {
	return &DynamoRepository{client: client, table: table, now: func() time.Time { return time.Now().UTC() }}
}

func (r *DynamoRepository) GetOrCreate(ctx context.Context, userID, email string) (Profile, error) {
	profile, found, err := r.get(ctx, userID)
	if err != nil || found {
		return profile, err
	}
	now := r.now()
	profile = Profile{UserID: userID, Email: email, CreatedAt: now, UpdatedAt: now}
	item, err := attributevalue.MarshalMap(profile)
	if err != nil {
		return Profile{}, err
	}
	_, err = r.client.PutItem(ctx, &dynamodb.PutItemInput{
		TableName: aws.String(r.table), Item: item,
		ConditionExpression: aws.String("attribute_not_exists(userId)"),
	})
	if err != nil {
		var conditional *types.ConditionalCheckFailedException
		if errors.As(err, &conditional) {
			return r.GetOrCreate(ctx, userID, email)
		}
		return Profile{}, err
	}
	return profile, nil
}

func (r *DynamoRepository) Update(ctx context.Context, userID, email string, update Update) (Profile, error) {
	existing, err := r.GetOrCreate(ctx, userID, email)
	if err != nil {
		return Profile{}, err
	}
	existing.Email = email
	existing.DisplayName = update.DisplayName
	existing.Locale = update.Locale
	existing.Timezone = update.Timezone
	existing.MarketingOptIn = update.MarketingOptIn
	existing.UpdatedAt = r.now()
	item, err := attributevalue.MarshalMap(existing)
	if err != nil {
		return Profile{}, err
	}
	_, err = r.client.PutItem(ctx, &dynamodb.PutItemInput{TableName: aws.String(r.table), Item: item})
	return existing, err
}

func (r *DynamoRepository) get(ctx context.Context, userID string) (Profile, bool, error) {
	output, err := r.client.GetItem(ctx, &dynamodb.GetItemInput{
		TableName: aws.String(r.table), ConsistentRead: aws.Bool(true),
		Key: map[string]types.AttributeValue{"userId": &types.AttributeValueMemberS{Value: userID}},
	})
	if err != nil {
		return Profile{}, false, err
	}
	if len(output.Item) == 0 {
		return Profile{}, false, nil
	}
	var profile Profile
	if err := attributevalue.UnmarshalMap(output.Item, &profile); err != nil {
		return Profile{}, false, err
	}
	return profile, true, nil
}
