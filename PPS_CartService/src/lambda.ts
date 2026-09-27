import type { APIGatewayProxyEventV2, Context } from "aws-lambda";
import serverless from "serverless-http";
import { createRuntime } from "./runtime.js";

const appHandler = serverless(createRuntime().app);

export function handler(event: APIGatewayProxyEventV2, context: Context) {
  context.callbackWaitsForEmptyEventLoop = false;
  const rawPath = event.rawPath.replace(/^\/api\/v1/, "");
  return appHandler({
    ...event,
    rawPath,
    requestContext: { ...event.requestContext, http: { ...event.requestContext.http, path: rawPath } },
  }, context);
}
