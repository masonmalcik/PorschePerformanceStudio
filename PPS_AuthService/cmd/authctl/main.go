package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"os"

	awsconfig "github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb"
	"github.com/porsche-performance-studio/pps-auth-service/internal/authz"
)

func main() {
	region := flag.String("region", value("AWS_REGION", "us-east-1"), "AWS region")
	table := flag.String("table", os.Getenv("DYNAMODB_TABLE"), "DynamoDB auth table")
	userID := flag.String("user-id", "", "Cognito subject to receive the role")
	roleID := flag.String("role", "administrator", "role to grant")
	flag.Parse()
	if *table == "" || *userID == "" || !authz.ValidID(*roleID) {
		log.Fatal("--table, --user-id, and a valid --role are required")
	}
	ctx := context.Background()
	cfg, err := awsconfig.LoadDefaultConfig(ctx, awsconfig.WithRegion(*region))
	if err != nil {
		log.Fatal(err)
	}
	repository := authz.New(dynamodb.NewFromConfig(cfg), *table)
	if err := repository.AssignRole(ctx, "system:authctl", *userID, *roleID, "", "authctl"); err != nil {
		log.Fatal(err)
	}
	fmt.Printf("granted role %s to user %s\n", *roleID, *userID)
}

func value(name, fallback string) string {
	if current := os.Getenv(name); current != "" {
		return current
	}
	return fallback
}
