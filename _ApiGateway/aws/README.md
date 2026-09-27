# PPS AWS gateway and messaging topology

This SAM template is the code-first definition for the eventual AWS demo deployment. It is intentionally not deployed until the Cart, Order, and Payment Lambda functions exist.

The cloud topology uses an AWS HTTP API with Cognito JWT authorization, one SNS topic, filtered SQS subscriptions, and a dedicated dead-letter queue for every consumer. AWS-owned encryption avoids customer-managed KMS key charges.

RabbitMQ remains the local Docker transport. Application code selects the adapter through `MESSAGE_TRANSPORT=rabbitmq` locally and `MESSAGE_TRANSPORT=aws` in Lambda. Event names and payloads remain identical across transports.

Validate without deploying:

```powershell
& "C:\Program Files\Amazon\AWSSAMCLI\bin\sam.cmd" validate --lint --template-file .\aws\template.yaml
```

When the service Lambdas have been deployed, supply their function ARNs as parameters to `sam deploy`. The payment webhook route is intentionally public because Stripe authenticates it cryptographically using `Stripe-Signature`; all user-facing Cart, Order, and Payment routes require Cognito JWTs.
