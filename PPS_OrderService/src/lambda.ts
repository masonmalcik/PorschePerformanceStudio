import type { APIGatewayProxyEventV2, Context, SQSEvent } from "aws-lambda";
import serverless from "serverless-http";
import { createRuntime } from "./runtime.js";
const runtime = createRuntime();
const http = serverless(runtime.app);
export function handler(event: APIGatewayProxyEventV2, context: Context) { context.callbackWaitsForEmptyEventLoop = false; const rawPath = event.rawPath.replace(/^\/api\/v1/, ""); return http({ ...event, rawPath, requestContext: { ...event.requestContext, http: { ...event.requestContext.http, path: rawPath } } }, context); }
export const events = (event: SQSEvent) => runtime.sqsHandler(event);
export async function publishOutbox(): Promise<void> { await runtime.outbox.runOnce(); }
