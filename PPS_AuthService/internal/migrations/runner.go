package migrations

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/feature/dynamodb/attributevalue"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb/types"
	"github.com/porsche-performance-studio/pps-auth-service/internal/authmodel"
)

type DynamoDBAPI interface {
	DeleteItem(context.Context, *dynamodb.DeleteItemInput, ...func(*dynamodb.Options)) (*dynamodb.DeleteItemOutput, error)
	GetItem(context.Context, *dynamodb.GetItemInput, ...func(*dynamodb.Options)) (*dynamodb.GetItemOutput, error)
	PutItem(context.Context, *dynamodb.PutItemInput, ...func(*dynamodb.Options)) (*dynamodb.PutItemOutput, error)
	UpdateItem(context.Context, *dynamodb.UpdateItemInput, ...func(*dynamodb.Options)) (*dynamodb.UpdateItemOutput, error)
}

type migration struct {
	id          string
	fingerprint string
	apply       func(context.Context, *Runner) error
}

type Runner struct {
	client DynamoDBAPI
	table  string
	now    func() time.Time
}

func New(client DynamoDBAPI, table string) *Runner {
	return &Runner{client: client, table: table, now: func() time.Time { return time.Now().UTC() }}
}

func (r *Runner) Up(ctx context.Context) error {
	owner := fmt.Sprintf("migration-%d", r.now().UnixNano())
	if err := r.acquireLock(ctx, owner); err != nil {
		return err
	}
	defer func() { _ = r.releaseLock(context.Background(), owner) }()

	for _, current := range definitions() {
		record, applied, err := r.applied(ctx, current.id)
		if err != nil {
			return err
		}
		checksum := checksum(current.fingerprint)
		if applied {
			if record.Checksum != checksum {
				return fmt.Errorf("migration %s checksum differs from the applied version", current.id)
			}
			continue
		}
		started := r.now()
		if err := current.apply(ctx, r); err != nil {
			return fmt.Errorf("apply migration %s: %w", current.id, err)
		}
		record = authmodel.MigrationRecord{
			PK: "SYSTEM", SK: "MIGRATION#" + current.id, EntityType: authmodel.EntityMigration,
			MigrationID: current.id, Checksum: checksum, Status: "applied", AppliedAt: r.now(),
			DurationMS: r.now().Sub(started).Milliseconds(),
		}
		if err := r.put(ctx, record, true); err != nil {
			return fmt.Errorf("record migration %s: %w", current.id, err)
		}
	}
	return nil
}

func definitions() []migration {
	return []migration{
		{id: "0001-schema-baseline", fingerprint: "composite PK/SK with GSI1", apply: func(context.Context, *Runner) error { return nil }},
		{id: "0002-seed-permissions", fingerprint: "permissions-v1", apply: seedPermissions},
		{id: "0003-seed-system-roles", fingerprint: "roles-v1", apply: seedRoles},
		{id: "0004-map-role-permissions", fingerprint: "role-permissions-v1", apply: seedRolePermissions},
		{id: "0005-authorization-api-permissions", fingerprint: "authorization-api-permissions-v1", apply: seedAuthorizationAPIPermissions},
	}
}

func seedAuthorizationAPIPermissions(ctx context.Context, r *Runner) error {
	additional := []struct{ code, description string }{
		{"organizations:read", "Read PPS organizations"},
		{"organizations:manage", "Create and update PPS organizations"},
		{"organizations:members:manage", "Manage organization memberships"},
	}
	for _, value := range additional {
		item := authmodel.Permission{PK: "PERMISSION#" + value.code, SK: "METADATA", EntityType: authmodel.EntityPermission, Code: value.code, Description: value.description}
		if err := r.putSeed(ctx, item); err != nil {
			return err
		}
	}

	additionalMappings := map[string][]string{
		"service-advisor": {"organizations:read"},
		"administrator":   {"organizations:read", "organizations:manage", "organizations:members:manage"},
	}
	now := r.now()
	for roleID, values := range additionalMappings {
		for _, permissionID := range values {
			item := authmodel.RolePermission{PK: "ROLE#" + roleID, SK: "PERMISSION#" + permissionID, EntityType: authmodel.EntityRolePermission, RoleID: roleID, PermissionID: permissionID, GrantedAt: now}
			if err := r.putSeed(ctx, item); err != nil {
				return err
			}
		}
	}

	for _, role := range roles {
		if err := r.updateRoleIndex(ctx, role.id); err != nil {
			return err
		}
	}
	return nil
}

func (r *Runner) updateRoleIndex(ctx context.Context, roleID string) error {
	for attempt := 0; attempt < 8; attempt++ {
		_, err := r.client.UpdateItem(ctx, &dynamodb.UpdateItemInput{
			TableName: aws.String(r.table), Key: map[string]types.AttributeValue{
				"PK": &types.AttributeValueMemberS{Value: "ROLE#" + roleID}, "SK": &types.AttributeValueMemberS{Value: "METADATA"},
			},
			UpdateExpression: aws.String("SET GSI1PK = :pk, GSI1SK = :sk"),
			ExpressionAttributeValues: map[string]types.AttributeValue{
				":pk": &types.AttributeValueMemberS{Value: "ENTITY#ROLE"}, ":sk": &types.AttributeValueMemberS{Value: roleID},
			},
			ConditionExpression: aws.String("attribute_exists(PK)"),
		})
		if err == nil {
			time.Sleep(1100 * time.Millisecond)
			return nil
		}
		if !isThrottled(err) {
			return err
		}
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-time.After(time.Duration(attempt+1) * time.Second):
		}
	}
	return errors.New("DynamoDB remained throttled while indexing roles")
}

var permissions = []struct{ code, description string }{
	{"profile:read:self", "Read the signed-in user's profile"},
	{"profile:update:self", "Update the signed-in user's profile"},
	{"users:read", "Read PPS users"},
	{"users:update", "Update PPS users"},
	{"users:suspend", "Suspend or reactivate PPS users"},
	{"roles:read", "Read roles and permissions"},
	{"roles:assign", "Assign roles to users"},
	{"roles:manage", "Create and update roles"},
	{"catalog:read", "Read the PPS catalog"},
	{"catalog:write", "Manage the PPS catalog"},
	{"vehicles:read:self", "Read the signed-in user's vehicles"},
	{"vehicles:manage:self", "Manage the signed-in user's vehicles"},
	{"appointments:create", "Create service appointments"},
	{"appointments:manage", "Manage service appointments"},
	{"audit:read", "Read authorization audit events"},
}

var roles = []struct{ id, name, description string }{
	{"customer", "Customer", "Standard PPS customer access"},
	{"technician", "Technician", "Workshop technician access"},
	{"service-advisor", "Service Advisor", "Customer and appointment management access"},
	{"catalog-manager", "Catalog Manager", "Catalog administration access"},
	{"administrator", "Administrator", "Full PPS authorization administration"},
}

var rolePermissions = map[string][]string{
	"customer":        {"profile:read:self", "profile:update:self", "catalog:read", "vehicles:read:self", "vehicles:manage:self", "appointments:create"},
	"technician":      {"profile:read:self", "profile:update:self", "catalog:read", "appointments:manage"},
	"service-advisor": {"profile:read:self", "profile:update:self", "users:read", "catalog:read", "appointments:create", "appointments:manage"},
	"catalog-manager": {"profile:read:self", "profile:update:self", "catalog:read", "catalog:write"},
	"administrator":   {"profile:read:self", "profile:update:self", "users:read", "users:update", "users:suspend", "roles:read", "roles:assign", "roles:manage", "catalog:read", "catalog:write", "appointments:manage", "audit:read"},
}

func seedPermissions(ctx context.Context, r *Runner) error {
	for _, value := range permissions {
		item := authmodel.Permission{PK: "PERMISSION#" + value.code, SK: "METADATA", EntityType: authmodel.EntityPermission, Code: value.code, Description: value.description}
		if err := r.putSeed(ctx, item); err != nil {
			return err
		}
	}
	return nil
}

func seedRoles(ctx context.Context, r *Runner) error {
	now := r.now()
	for _, value := range roles {
		item := authmodel.Role{PK: "ROLE#" + value.id, SK: "METADATA", EntityType: authmodel.EntityRole, RoleID: value.id, Name: value.name, Description: value.description, System: true, CreatedAt: now, UpdatedAt: now}
		if err := r.putSeed(ctx, item); err != nil {
			return err
		}
	}
	return nil
}

func seedRolePermissions(ctx context.Context, r *Runner) error {
	now := r.now()
	for roleID, values := range rolePermissions {
		for _, permissionID := range values {
			item := authmodel.RolePermission{PK: "ROLE#" + roleID, SK: "PERMISSION#" + permissionID, EntityType: authmodel.EntityRolePermission, RoleID: roleID, PermissionID: permissionID, GrantedAt: now}
			if err := r.putSeed(ctx, item); err != nil {
				return err
			}
		}
	}
	return nil
}

func (r *Runner) putSeed(ctx context.Context, value any) error {
	for attempt := 0; attempt < 8; attempt++ {
		err := r.put(ctx, value, true)
		if err == nil || isConditional(err) {
			time.Sleep(1100 * time.Millisecond)
			return nil
		}
		if !isThrottled(err) {
			return err
		}
		delay := time.Duration(attempt+1) * time.Second
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-time.After(delay):
		}
	}
	return errors.New("DynamoDB remained throttled after migration retries")
}

func (r *Runner) put(ctx context.Context, value any, createOnly bool) error {
	item, err := attributevalue.MarshalMap(value)
	if err != nil {
		return err
	}
	input := &dynamodb.PutItemInput{TableName: aws.String(r.table), Item: item}
	if createOnly {
		input.ConditionExpression = aws.String("attribute_not_exists(PK)")
	}
	_, err = r.client.PutItem(ctx, input)
	return err
}

func (r *Runner) applied(ctx context.Context, id string) (authmodel.MigrationRecord, bool, error) {
	output, err := r.client.GetItem(ctx, &dynamodb.GetItemInput{
		TableName: aws.String(r.table), ConsistentRead: aws.Bool(true),
		Key: map[string]types.AttributeValue{"PK": &types.AttributeValueMemberS{Value: "SYSTEM"}, "SK": &types.AttributeValueMemberS{Value: "MIGRATION#" + id}},
	})
	if err != nil || len(output.Item) == 0 {
		return authmodel.MigrationRecord{}, false, err
	}
	var record authmodel.MigrationRecord
	if err := attributevalue.UnmarshalMap(output.Item, &record); err != nil {
		return authmodel.MigrationRecord{}, false, err
	}
	return record, true, nil
}

func (r *Runner) acquireLock(ctx context.Context, owner string) error {
	ttl := r.now().Add(5 * time.Minute).Unix()
	_, err := r.client.PutItem(ctx, &dynamodb.PutItemInput{
		TableName: aws.String(r.table),
		Item: map[string]types.AttributeValue{
			"PK": &types.AttributeValueMemberS{Value: "SYSTEM"}, "SK": &types.AttributeValueMemberS{Value: "MIGRATION_LOCK"},
			"owner": &types.AttributeValueMemberS{Value: owner}, "ttl": &types.AttributeValueMemberN{Value: fmt.Sprint(ttl)},
		},
		ConditionExpression:       aws.String("attribute_not_exists(PK) OR #ttl < :now"),
		ExpressionAttributeNames:  map[string]string{"#ttl": "ttl"},
		ExpressionAttributeValues: map[string]types.AttributeValue{":now": &types.AttributeValueMemberN{Value: fmt.Sprint(r.now().Unix())}},
	})
	if isConditional(err) {
		return errors.New("another migration process holds the migration lock")
	}
	return err
}

func (r *Runner) releaseLock(ctx context.Context, owner string) error {
	_, err := r.client.DeleteItem(ctx, &dynamodb.DeleteItemInput{
		TableName:           aws.String(r.table),
		Key:                 map[string]types.AttributeValue{"PK": &types.AttributeValueMemberS{Value: "SYSTEM"}, "SK": &types.AttributeValueMemberS{Value: "MIGRATION_LOCK"}},
		ConditionExpression: aws.String("#owner = :owner"), ExpressionAttributeNames: map[string]string{"#owner": "owner"},
		ExpressionAttributeValues: map[string]types.AttributeValue{":owner": &types.AttributeValueMemberS{Value: owner}},
	})
	return err
}

func checksum(value string) string {
	sum := sha256.Sum256([]byte(value))
	return hex.EncodeToString(sum[:])
}

func isConditional(err error) bool {
	var conditional *types.ConditionalCheckFailedException
	return errors.As(err, &conditional)
}

func isThrottled(err error) bool {
	var exceeded *types.ProvisionedThroughputExceededException
	return errors.As(err, &exceeded)
}
