package main

import (
	"context"
	"encoding/base64"
	"log"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"time"

	"github.com/aws/aws-lambda-go/events"
	"github.com/aws/aws-lambda-go/lambda"
	awsconfig "github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb"
	appconfig "github.com/porsche-performance-studio/pps-auth-service/internal/config"
	"github.com/porsche-performance-studio/pps-auth-service/internal/httpapi"
	"github.com/porsche-performance-studio/pps-auth-service/internal/lambdaadapter"
	"github.com/porsche-performance-studio/pps-auth-service/internal/profile"
)

var handler http.Handler

func init() {
	cfg, err := appconfig.Load()
	if err != nil {
		log.Fatal(err)
	}
	awsCfg, err := awsconfig.LoadDefaultConfig(context.Background(), awsconfig.WithRegion(cfg.Region))
	if err != nil {
		log.Fatal(err)
	}
	handler = httpapi.New(cfg, profile.NewDynamoRepository(dynamodb.NewFromConfig(awsCfg), cfg.DynamoDBTable), &http.Client{Timeout: 10 * time.Second}, slog.New(slog.NewJSONHandler(os.Stdout, nil)))
}

func main() { lambda.Start(invoke) }

func invoke(ctx context.Context, event events.APIGatewayV2HTTPRequest) (events.APIGatewayV2HTTPResponse, error) {
	body := event.Body
	if event.IsBase64Encoded {
		decoded, err := base64.StdEncoding.DecodeString(body)
		if err != nil {
			return events.APIGatewayV2HTTPResponse{StatusCode: 400}, nil
		}
		body = string(decoded)
	}
	request, err := http.NewRequestWithContext(ctx, event.RequestContext.HTTP.Method, "https://lambda.local"+lambdaadapter.RoutePath(event.RawPath, event.RequestContext.Stage)+query(event.RawQueryString), strings.NewReader(body))
	if err != nil {
		return events.APIGatewayV2HTTPResponse{}, err
	}
	for key, value := range event.Headers {
		request.Header.Set(key, value)
	}
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, request)
	response := recorder.Result()
	defer response.Body.Close()
	return events.APIGatewayV2HTTPResponse{StatusCode: response.StatusCode, Headers: flatten(response.Header), Body: recorder.Body.String()}, nil
}

func query(raw string) string {
	if raw == "" {
		return ""
	}
	return "?" + raw
}

func flatten(headers http.Header) map[string]string {
	values := map[string]string{}
	for key, list := range headers {
		values[key] = strings.Join(list, ", ")
	}
	return values
}
