package authz

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"sort"
	"strings"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/feature/dynamodb/attributevalue"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb/types"
	"github.com/porsche-performance-studio/pps-auth-service/internal/authmodel"
)

var (
	ErrConflict = errors.New("authorization record already exists")
	ErrNotFound = errors.New("authorization record not found")
)

type DynamoDBAPI interface {
	GetItem(context.Context, *dynamodb.GetItemInput, ...func(*dynamodb.Options)) (*dynamodb.GetItemOutput, error)
	Query(context.Context, *dynamodb.QueryInput, ...func(*dynamodb.Options)) (*dynamodb.QueryOutput, error)
	TransactWriteItems(context.Context, *dynamodb.TransactWriteItemsInput, ...func(*dynamodb.Options)) (*dynamodb.TransactWriteItemsOutput, error)
}

type Repository interface {
	Permissions(context.Context, string) (map[string]struct{}, error)
	ListRoles(context.Context) ([]authmodel.Role, error)
	GetRole(context.Context, string) (authmodel.Role, []string, error)
	CreateRole(context.Context, string, string, string, string) (authmodel.Role, error)
	SetRolePermissions(context.Context, string, string, []string, string) error
	AssignRole(context.Context, string, string, string, string, string) error
	RemoveRole(context.Context, string, string, string, string, string) error
	ListOrganizations(context.Context) ([]authmodel.Organization, error)
	CreateOrganization(context.Context, string, string, string) (authmodel.Organization, error)
	UpsertMembership(context.Context, string, string, string, string, string) (authmodel.Membership, error)
	ListConsents(context.Context, string) ([]authmodel.Consent, error)
	RecordConsent(context.Context, string, string, string, bool, string, string) (authmodel.Consent, error)
}

type DynamoRepository struct {
	client DynamoDBAPI
	table  string
	now    func() time.Time
}

func New(client DynamoDBAPI, table string) *DynamoRepository {
	return &DynamoRepository{client: client, table: table, now: func() time.Time { return time.Now().UTC() }}
}

func (r *DynamoRepository) Permissions(ctx context.Context, userID string) (map[string]struct{}, error) {
	output, err := r.queryPartition(ctx, "USER#"+userID)
	if err != nil {
		return nil, err
	}
	roleIDs := map[string]struct{}{}
	active := false
	for _, item := range output {
		switch stringValue(item["entityType"]) {
		case authmodel.EntityUserProfile:
			active = stringValue(item["status"]) == string(authmodel.UserStatusActive)
		case authmodel.EntityUserRole:
			roleIDs[stringValue(item["roleId"])] = struct{}{}
		}
	}
	if !active {
		return map[string]struct{}{}, nil
	}
	permissions := map[string]struct{}{}
	for roleID := range roleIDs {
		items, err := r.queryPartition(ctx, "ROLE#"+roleID)
		if err != nil {
			return nil, err
		}
		for _, item := range items {
			if stringValue(item["entityType"]) == authmodel.EntityRolePermission {
				permissions[stringValue(item["permissionId"])] = struct{}{}
			}
		}
	}
	return permissions, nil
}

func (r *DynamoRepository) ListRoles(ctx context.Context) ([]authmodel.Role, error) {
	items, err := r.queryIndex(ctx, "ENTITY#ROLE")
	if err != nil {
		return nil, err
	}
	roles := make([]authmodel.Role, 0, len(items))
	for _, item := range items {
		var role authmodel.Role
		if err := attributevalue.UnmarshalMap(item, &role); err != nil {
			return nil, err
		}
		roles = append(roles, role)
	}
	sort.Slice(roles, func(i, j int) bool { return roles[i].RoleID < roles[j].RoleID })
	return roles, nil
}

func (r *DynamoRepository) GetRole(ctx context.Context, roleID string) (authmodel.Role, []string, error) {
	items, err := r.queryPartition(ctx, "ROLE#"+roleID)
	if err != nil {
		return authmodel.Role{}, nil, err
	}
	var role authmodel.Role
	permissions := []string{}
	for _, item := range items {
		switch stringValue(item["entityType"]) {
		case authmodel.EntityRole:
			if err := attributevalue.UnmarshalMap(item, &role); err != nil {
				return authmodel.Role{}, nil, err
			}
		case authmodel.EntityRolePermission:
			permissions = append(permissions, stringValue(item["permissionId"]))
		}
	}
	if role.RoleID == "" {
		return authmodel.Role{}, nil, ErrNotFound
	}
	sort.Strings(permissions)
	return role, permissions, nil
}

func (r *DynamoRepository) CreateRole(ctx context.Context, actorID, roleID, name, description string) (authmodel.Role, error) {
	now := r.now()
	role := authmodel.Role{
		PK: "ROLE#" + roleID, SK: "METADATA", GSI1PK: "ENTITY#ROLE", GSI1SK: roleID,
		EntityType: authmodel.EntityRole, RoleID: roleID, Name: name, Description: description,
		System: false, CreatedAt: now, UpdatedAt: now,
	}
	roleItem, err := marshal(role)
	if err != nil {
		return authmodel.Role{}, err
	}
	audit, err := r.auditItem(actorID, "role.create", "role", roleID, "")
	if err != nil {
		return authmodel.Role{}, err
	}
	err = r.transact(ctx,
		transactPut(r.table, roleItem, "attribute_not_exists(PK)"),
		transactPut(r.table, audit, "attribute_not_exists(PK)"),
	)
	if isCanceled(err) {
		return authmodel.Role{}, ErrConflict
	}
	return role, err
}

func (r *DynamoRepository) SetRolePermissions(ctx context.Context, actorID, roleID string, permissions []string, requestID string) error {
	_, existing, err := r.GetRole(ctx, roleID)
	if err != nil {
		return err
	}
	now := r.now()
	items := make([]types.TransactWriteItem, 0, len(existing)+len(permissions)+1)
	for _, permission := range existing {
		items = append(items, types.TransactWriteItem{Delete: &types.Delete{TableName: aws.String(r.table), Key: key("ROLE#"+roleID, "PERMISSION#"+permission)}})
	}
	seen := map[string]struct{}{}
	for _, permission := range permissions {
		if _, found := seen[permission]; found {
			continue
		}
		seen[permission] = struct{}{}
		items = append(items, types.TransactWriteItem{ConditionCheck: &types.ConditionCheck{
			TableName: aws.String(r.table), Key: key("PERMISSION#"+permission, "METADATA"), ConditionExpression: aws.String("attribute_exists(PK)"),
		}})
		value := authmodel.RolePermission{PK: "ROLE#" + roleID, SK: "PERMISSION#" + permission, EntityType: authmodel.EntityRolePermission, RoleID: roleID, PermissionID: permission, GrantedAt: now}
		item, err := marshal(value)
		if err != nil {
			return err
		}
		items = append(items, transactPut(r.table, item, ""))
	}
	audit, err := r.auditItem(actorID, "role.permissions.update", "role", roleID, requestID)
	if err != nil {
		return err
	}
	items = append(items, transactPut(r.table, audit, "attribute_not_exists(PK)"))
	if len(items) > 100 {
		return errors.New("role permission update exceeds DynamoDB transaction limit")
	}
	return r.transact(ctx, items...)
}

func (r *DynamoRepository) AssignRole(ctx context.Context, actorID, userID, roleID, organizationID, requestID string) error {
	now := r.now()
	sk := "ROLE#" + roleID
	if organizationID != "" {
		sk = "ORG#" + organizationID + "#ROLE#" + roleID
	}
	assignment := authmodel.UserRole{
		PK: "USER#" + userID, SK: sk, GSI1PK: "ROLE#" + roleID, GSI1SK: "USER#" + userID,
		EntityType: authmodel.EntityUserRole, UserID: userID, RoleID: roleID, OrganizationID: organizationID,
		GrantedBy: actorID, GrantedAt: now,
	}
	item, err := marshal(assignment)
	if err != nil {
		return err
	}
	audit, err := r.auditItem(actorID, "user.role.assign", "user", userID, requestID)
	if err != nil {
		return err
	}
	return r.transact(ctx,
		types.TransactWriteItem{ConditionCheck: &types.ConditionCheck{TableName: aws.String(r.table), Key: key("ROLE#"+roleID, "METADATA"), ConditionExpression: aws.String("attribute_exists(PK)")}},
		types.TransactWriteItem{ConditionCheck: &types.ConditionCheck{TableName: aws.String(r.table), Key: key("USER#"+userID, "PROFILE"), ConditionExpression: aws.String("attribute_exists(PK)")}},
		transactPut(r.table, item, ""), transactPut(r.table, audit, "attribute_not_exists(PK)"),
	)
}

func (r *DynamoRepository) RemoveRole(ctx context.Context, actorID, userID, roleID, organizationID, requestID string) error {
	sk := "ROLE#" + roleID
	if organizationID != "" {
		sk = "ORG#" + organizationID + "#ROLE#" + roleID
	}
	audit, err := r.auditItem(actorID, "user.role.remove", "user", userID, requestID)
	if err != nil {
		return err
	}
	return r.transact(ctx,
		types.TransactWriteItem{Delete: &types.Delete{TableName: aws.String(r.table), Key: key("USER#"+userID, sk)}},
		transactPut(r.table, audit, "attribute_not_exists(PK)"),
	)
}

func (r *DynamoRepository) ListOrganizations(ctx context.Context) ([]authmodel.Organization, error) {
	items, err := r.queryIndex(ctx, "ENTITY#ORGANIZATION")
	if err != nil {
		return nil, err
	}
	values := make([]authmodel.Organization, 0, len(items))
	for _, item := range items {
		var value authmodel.Organization
		if err := attributevalue.UnmarshalMap(item, &value); err != nil {
			return nil, err
		}
		values = append(values, value)
	}
	sort.Slice(values, func(i, j int) bool { return values[i].OrganizationID < values[j].OrganizationID })
	return values, nil
}

func (r *DynamoRepository) CreateOrganization(ctx context.Context, actorID, name, requestID string) (authmodel.Organization, error) {
	id, err := randomID()
	if err != nil {
		return authmodel.Organization{}, err
	}
	now := r.now()
	value := authmodel.Organization{
		PK: "ORG#" + id, SK: "METADATA", GSI1PK: "ENTITY#ORGANIZATION", GSI1SK: id,
		EntityType: authmodel.EntityOrganization, OrganizationID: id, Name: name, Status: "active", CreatedAt: now, UpdatedAt: now,
	}
	item, err := marshal(value)
	if err != nil {
		return authmodel.Organization{}, err
	}
	audit, err := r.auditItem(actorID, "organization.create", "organization", id, requestID)
	if err != nil {
		return authmodel.Organization{}, err
	}
	err = r.transact(ctx, transactPut(r.table, item, "attribute_not_exists(PK)"), transactPut(r.table, audit, "attribute_not_exists(PK)"))
	return value, err
}

func (r *DynamoRepository) UpsertMembership(ctx context.Context, actorID, organizationID, userID, status, requestID string) (authmodel.Membership, error) {
	value := authmodel.Membership{
		PK: "ORG#" + organizationID, SK: "MEMBER#" + userID, GSI1PK: "USER#" + userID, GSI1SK: "ORG#" + organizationID,
		EntityType: authmodel.EntityMembership, OrganizationID: organizationID, UserID: userID, Status: status, JoinedAt: r.now(),
	}
	item, err := marshal(value)
	if err != nil {
		return authmodel.Membership{}, err
	}
	audit, err := r.auditItem(actorID, "organization.membership.upsert", "organization", organizationID, requestID)
	if err != nil {
		return authmodel.Membership{}, err
	}
	err = r.transact(ctx,
		types.TransactWriteItem{ConditionCheck: &types.ConditionCheck{TableName: aws.String(r.table), Key: key("ORG#"+organizationID, "METADATA"), ConditionExpression: aws.String("attribute_exists(PK)")}},
		types.TransactWriteItem{ConditionCheck: &types.ConditionCheck{TableName: aws.String(r.table), Key: key("USER#"+userID, "PROFILE"), ConditionExpression: aws.String("attribute_exists(PK)")}},
		transactPut(r.table, item, ""), transactPut(r.table, audit, "attribute_not_exists(PK)"),
	)
	return value, err
}

func (r *DynamoRepository) ListConsents(ctx context.Context, userID string) ([]authmodel.Consent, error) {
	items, err := r.queryPrefix(ctx, "USER#"+userID, "CONSENT#")
	if err != nil {
		return nil, err
	}
	values := make([]authmodel.Consent, 0, len(items))
	for _, item := range items {
		var value authmodel.Consent
		if err := attributevalue.UnmarshalMap(item, &value); err != nil {
			return nil, err
		}
		values = append(values, value)
	}
	return values, nil
}

func (r *DynamoRepository) RecordConsent(ctx context.Context, userID, consentType, policyVersion string, granted bool, source, requestID string) (authmodel.Consent, error) {
	now := r.now()
	id, err := randomID()
	if err != nil {
		return authmodel.Consent{}, err
	}
	value := authmodel.Consent{
		PK: "USER#" + userID, SK: "CONSENT#" + consentType + "#" + now.Format(time.RFC3339Nano) + "#" + id,
		EntityType: authmodel.EntityConsent, UserID: userID, ConsentType: consentType, PolicyVersion: policyVersion,
		Granted: granted, RecordedAt: now, Source: source,
	}
	item, err := marshal(value)
	if err != nil {
		return authmodel.Consent{}, err
	}
	audit, err := r.auditItem(userID, "consent.record", "user", userID, requestID)
	if err != nil {
		return authmodel.Consent{}, err
	}
	err = r.transact(ctx, transactPut(r.table, item, "attribute_not_exists(PK)"), transactPut(r.table, audit, "attribute_not_exists(PK)"))
	return value, err
}

func (r *DynamoRepository) queryPartition(ctx context.Context, pk string) ([]map[string]types.AttributeValue, error) {
	return r.query(ctx, &dynamodb.QueryInput{
		TableName: aws.String(r.table), KeyConditionExpression: aws.String("PK = :pk"),
		ExpressionAttributeValues: map[string]types.AttributeValue{":pk": &types.AttributeValueMemberS{Value: pk}},
	})
}

func (r *DynamoRepository) queryPrefix(ctx context.Context, pk, prefix string) ([]map[string]types.AttributeValue, error) {
	return r.query(ctx, &dynamodb.QueryInput{
		TableName: aws.String(r.table), KeyConditionExpression: aws.String("PK = :pk AND begins_with(SK, :prefix)"),
		ExpressionAttributeValues: map[string]types.AttributeValue{":pk": &types.AttributeValueMemberS{Value: pk}, ":prefix": &types.AttributeValueMemberS{Value: prefix}},
	})
}

func (r *DynamoRepository) queryIndex(ctx context.Context, pk string) ([]map[string]types.AttributeValue, error) {
	return r.query(ctx, &dynamodb.QueryInput{
		TableName: aws.String(r.table), IndexName: aws.String("GSI1"), KeyConditionExpression: aws.String("GSI1PK = :pk"),
		ExpressionAttributeValues: map[string]types.AttributeValue{":pk": &types.AttributeValueMemberS{Value: pk}},
	})
}

func (r *DynamoRepository) query(ctx context.Context, input *dynamodb.QueryInput) ([]map[string]types.AttributeValue, error) {
	items := []map[string]types.AttributeValue{}
	for {
		output, err := r.client.Query(ctx, input)
		if err != nil {
			return nil, err
		}
		items = append(items, output.Items...)
		if len(output.LastEvaluatedKey) == 0 {
			return items, nil
		}
		input.ExclusiveStartKey = output.LastEvaluatedKey
	}
}

func (r *DynamoRepository) auditItem(actorID, action, resourceType, resourceID, requestID string) (map[string]types.AttributeValue, error) {
	id, err := randomID()
	if err != nil {
		return nil, err
	}
	now := r.now()
	value := authmodel.AuditEvent{
		PK: "AUDIT#" + now.Format("2006-01"), SK: now.Format(time.RFC3339Nano) + "#" + id,
		EntityType: authmodel.EntityAuditEvent, EventID: id, ActorUserID: actorID, Action: action,
		ResourceType: resourceType, ResourceID: resourceID, Timestamp: now, RequestID: requestID,
	}
	return marshal(value)
}

func (r *DynamoRepository) transact(ctx context.Context, items ...types.TransactWriteItem) error {
	_, err := r.client.TransactWriteItems(ctx, &dynamodb.TransactWriteItemsInput{TransactItems: items})
	return err
}

func marshal(value any) (map[string]types.AttributeValue, error) {
	return attributevalue.MarshalMap(value)
}

func key(pk, sk string) map[string]types.AttributeValue {
	return map[string]types.AttributeValue{"PK": &types.AttributeValueMemberS{Value: pk}, "SK": &types.AttributeValueMemberS{Value: sk}}
}

func transactPut(table string, item map[string]types.AttributeValue, condition string) types.TransactWriteItem {
	put := &types.Put{TableName: aws.String(table), Item: item}
	if condition != "" {
		put.ConditionExpression = aws.String(condition)
	}
	return types.TransactWriteItem{Put: put}
}

func randomID() (string, error) {
	value := make([]byte, 16)
	if _, err := rand.Read(value); err != nil {
		return "", err
	}
	return hex.EncodeToString(value), nil
}

func stringValue(value types.AttributeValue) string {
	if current, ok := value.(*types.AttributeValueMemberS); ok {
		return current.Value
	}
	return ""
}

func isCanceled(err error) bool {
	var canceled *types.TransactionCanceledException
	return errors.As(err, &canceled)
}

func ValidID(value string) bool {
	if value == "" || len(value) > 100 {
		return false
	}
	for _, current := range value {
		if !(current >= 'a' && current <= 'z') && !(current >= '0' && current <= '9') && current != '-' && current != ':' {
			return false
		}
	}
	return !strings.Contains(value, "##")
}
