# Catalog deployment

The development stack is `pps-catalog-dev` in `us-east-1`. It deploys the ARM64 Lambda API, HTTP API, a private versioned S3 asset source, and CloudWatch alarms. Public assets are packaged with the Lambda and served under `/assets` with cache headers. CloudFront can be added in front of the private bucket after AWS Support verifies this account for CloudFront resource creation.

GitHub Actions uses OIDC instead of long-lived AWS access keys. Configure the `development` GitHub environment with these repository variables:

- `AWS_DEPLOY_ROLE_ARN`: IAM role trusted only by `masonmalcik/PorschePerformanceStudio`
- `MONGODB_SECRET_ARN`: Secrets Manager ARN containing the `MONGODB_URI` JSON property

Redis and Elasticsearch remain optional. Empty endpoints deliberately activate the tested MongoDB fail-open path and avoid recurring cache/search infrastructure costs.

The Atlas `0.0.0.0/0` development access-list entry must be replaced before production. A fixed-egress NAT Gateway has a continuous hourly charge; use Atlas private networking or a cost-reviewed fixed-egress design for production.
