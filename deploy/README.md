# PPS AWS deployment

The unified SAM stack in `_ApiGateway/aws/template.yaml` owns the HTTP API, Lambda functions, SNS topic, SQS queues and dead-letter queues, event-source mappings, and scheduled outbox publishers.

## Required Secrets Manager JSON secrets

- `pps/order/dev`: `{ "DATABASE_URL": "postgresql://..." }`
- `pps/payment/dev`: `{ "DATABASE_URL": "postgresql://..." }`
- `pps/stripe/dev`: `{ "STRIPE_SECRET_KEY": "sk_test_...", "STRIPE_WEBHOOK_SECRET": "whsec_..." }`

The stack resolves these values at deployment time. Secret values must never be committed or passed as CloudFormation parameters.

## Build and validate

```powershell
& .\deploy\build-lambda-artifacts.ps1
& "C:\Program Files\Amazon\AWSSAMCLI\bin\sam.cmd" validate --lint --template-file .\_ApiGateway\aws\template.yaml
```

The artifacts are deliberately prebuilt: the script generates the Amazon Linux Prisma engine and materializes the local messaging package. Do not run `sam build` over them on Windows, because its npm builder replaces those deployment artifacts. Apply the Order and Payment Prisma migrations before deploying, then run `sam deploy --guided --template-file .\_ApiGateway\aws\template.yaml`. SAM packages the prebuilt `CodeUri` directories and uploads them to its managed S3 bucket.
