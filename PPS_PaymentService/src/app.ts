import cors from "cors";
import express, { type NextFunction, type Request, type Response } from "express";
import helmet from "helmet";
import { pinoHttp } from "pino-http";
import { z } from "zod";
import { IdempotencyConflictError, PaymentProviderError } from "./domain/payment.js";
import type { AppLogger, StripeGateway } from "./ports.js";
import type { PaymentService } from "./services/payment-service.js";

const createSchema = z.object({
  orderId: z.string().uuid(),
  amount: z.number().int().positive().max(99_999_999),
  currency: z.string().length(3).regex(/^[A-Za-z]{3}$/).transform((value) => value.toLowerCase()),
}).strict();
const safeId = z.string().trim().min(8).max(128).regex(/^[A-Za-z0-9._:@-]+$/);

export function createApp(options: {
  service: PaymentService;
  stripe: StripeGateway;
  logger: AppLogger;
  trustedUserHeader?: string;
  corsOrigin?: string;
}): express.Express {
  const app = express();
  const identityHeader = options.trustedUserHeader ?? "x-authenticated-user-id";
  app.disable("x-powered-by");
  app.use(helmet());
  app.use(pinoHttp({ logger: options.logger as never }));

  app.post("/payments/webhook", express.raw({ type: "application/json", limit: "1mb" }), asyncRoute(async (request, response) => {
    const signature = request.header("stripe-signature");
    if (!signature || !Buffer.isBuffer(request.body)) {
      response.status(400).json({ error: "invalid_webhook_signature" });
      return;
    }
    try {
      const event = options.stripe.constructWebhookEvent(request.body, signature);
      const result = await options.service.processWebhook(event);
      response.status(200).json({ received: true, duplicate: result.duplicate });
    } catch (error) {
      if (isSignatureError(error)) {
        options.logger.warn({ error }, "Stripe webhook signature verification failed");
        response.status(400).json({ error: "invalid_webhook_signature" });
        return;
      }
      throw error;
    }
  }));

  app.use(cors({ origin: options.corsOrigin ?? "http://localhost:3000", credentials: true }));
  app.use(express.json({ limit: "32kb" }));
  app.get("/health", (_request, response) => response.json({ status: "ok" }));

  app.post("/payments/create-intent", asyncRoute(async (request, response) => {
    const userId = request.header(identityHeader);
    if (!userId) {
      response.status(401).json({ error: "authentication_required" });
      return;
    }
    const idempotencyKey = request.header("idempotency-key") ?? request.header("idempotency_key");
    if (!idempotencyKey) {
      response.status(400).json({ error: "idempotency_key_required" });
      return;
    }
    const body = createSchema.parse(request.body);
    const result = await options.service.createIntent({
      ...body,
      userId: safeId.parse(userId),
      idempotencyKey: safeId.parse(idempotencyKey),
    });
    response.status(result.replayed ? 200 : 201)
      .set("Idempotency-Replayed", String(result.replayed))
      .json({ transaction: result.transaction, clientSecret: result.clientSecret });
  }));

  app.use((error: unknown, _request: Request, response: Response, _next: NextFunction) => {
    if (error instanceof z.ZodError) return response.status(400).json({ error: "invalid_request", details: error.issues });
    if (error instanceof IdempotencyConflictError) return response.status(409).json({ error: "idempotency_conflict" });
    if (error instanceof PaymentProviderError) return response.status(502).json({ error: "payment_provider_unavailable" });
    options.logger.error({ error }, "Unhandled payment request error");
    return response.status(500).json({ error: "internal_server_error" });
  });
  return app;
}

function asyncRoute(handler: (request: Request, response: Response) => Promise<void>): express.RequestHandler {
  return (request, response, next) => void handler(request, response).catch(next);
}

function isSignatureError(error: unknown): boolean {
  return error instanceof Error && (
    (error as Error & { type?: string }).type === "StripeSignatureVerificationError"
    || error.name === "StripeSignatureVerificationError"
  );
}
