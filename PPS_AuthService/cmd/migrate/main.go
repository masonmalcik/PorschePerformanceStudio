package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"os"

	awsconfig "github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb"
	"github.com/porsche-performance-studio/pps-auth-service/internal/migrations"
)

func main() {
	region := flag.String("region", value("AWS_REGION", "us-east-1"), "AWS region")
	table := flag.String("table", os.Getenv("DYNAMODB_TABLE"), "DynamoDB auth table")
	flag.Parse()
	if *table == "" {
		log.Fatal("--table or DYNAMODB_TABLE is required")
	}
	ctx := context.Background()
	cfg, err := awsconfig.LoadDefaultConfig(ctx, awsconfig.WithRegion(*region))
	if err != nil {
		log.Fatal(err)
	}
	if err := migrations.New(dynamodb.NewFromConfig(cfg), *table).Up(ctx); err != nil {
		log.Fatal(err)
	}
	fmt.Printf("auth migrations applied to %s in %s\n", *table, *region)
}

func value(name, fallback string) string {
	if current := os.Getenv(name); current != "" {
		return current
	}
	return fallback
}
