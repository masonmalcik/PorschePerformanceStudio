package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"log/slog"
	"net/http"
	"os"
	"time"

	awsconfig "github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb"
	"github.com/porsche-performance-studio/pps-auth-service/internal/authz"
	appconfig "github.com/porsche-performance-studio/pps-auth-service/internal/config"
	"github.com/porsche-performance-studio/pps-auth-service/internal/httpapi"
	"github.com/porsche-performance-studio/pps-auth-service/internal/profile"
)

func main() {
	healthcheck := flag.Bool("healthcheck", false, "check the local health endpoint")
	flag.Parse()
	if *healthcheck {
		checkHealth()
		return
	}
	cfg, err := appconfig.Load()
	if err != nil {
		log.Fatal(err)
	}
	awsCfg, err := awsconfig.LoadDefaultConfig(context.Background(), awsconfig.WithRegion(cfg.Region))
	if err != nil {
		log.Fatal(err)
	}
	repository := profile.NewDynamoRepository(dynamodb.NewFromConfig(awsCfg), cfg.DynamoDBTable)
	authorization := authz.New(dynamodb.NewFromConfig(awsCfg), cfg.DynamoDBTable)
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	server := &http.Server{Addr: ":" + cfg.Port, Handler: httpapi.New(cfg, repository, authorization, &http.Client{Timeout: 10 * time.Second}, logger), ReadHeaderTimeout: 5 * time.Second, ReadTimeout: 10 * time.Second, WriteTimeout: 15 * time.Second, IdleTimeout: 60 * time.Second}
	logger.Info("auth service listening", "port", cfg.Port)
	log.Fatal(server.ListenAndServe())
}

func checkHealth() {
	response, err := http.Get("http://127.0.0.1:8082/health")
	if err != nil || response.StatusCode != http.StatusOK {
		os.Exit(1)
	}
	_ = response.Body.Close()
	fmt.Println("healthy")
}
