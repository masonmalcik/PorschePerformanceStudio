import cors from "cors";
import express, { type NextFunction, type Request, type Response } from "express";
import helmet from "helmet";
import { pinoHttp } from "pino-http";
import { z } from "zod";
import { InvalidOrderTransitionError, OrderNotFoundError } from "./domain/order.js";
import type { AppLogger } from "./ports.js";
import type { OrderService } from "./services/order-service.js";

const id = z.string().trim().min(1).max(128).regex(/^[A-Za-z0-9._:@-]+$/);
const uuid = z.string().uuid();
const createBody = z.object({
  items: z.array(z.object({
    productId: id,
    quantity: z.number().int().min(1).max(999),
    pricePerUnit: z.string().regex(/^\d{1,12}(?:\.\d{1,2})?$/),
  }).strict()).min(1).max(100),
}).strict();

export function createApp(options: {
  service: OrderService;
  logger: AppLogger;
  trustedUserHeader?: string;
  corsOrigin?: string;
}): express.Express {
  const app = express();
  const identityHeader = options.trustedUserHeader ?? "x-authenticated-user-id";
  app.disable("x-powered-by");
  app.use(helmet());
  app.use(cors({ origin: options.corsOrigin ?? "http://localhost:3000", credentials: true }));
  app.use(express.json({ limit: "64kb" }));
  app.use(pinoHttp({ logger: options.logger as never }));
  app.get("/health", (_request, response) => response.json({ status: "ok" }));

  const principal = (request: Request): string => {
    const value = request.header(identityHeader);
    if (!value) throw new AuthenticationError();
    return id.parse(value);
  };

  app.post("/orders", route(async (request, response) => {
    const userId = principal(request);
    const idempotencyKey = request.header("idempotency_key") ?? request.header("idempotency-key");
    if (!idempotencyKey) throw new ValidationError("idempotency_key header is required");
    const key = id.parse(idempotencyKey);
    const body = createBody.parse(request.body);
    const result = await options.service.create({ userId, idempotencyKey: key, items: body.items });
    response.status(result.replayed ? 200 : 201).set("Idempotency-Replayed", String(result.replayed)).json(result.order);
  }));

  app.get("/orders/user/:userId", route(async (request, response) => {
    const userId = id.parse(request.params.userId);
    if (principal(request) !== userId) throw new AuthorizationError();
    response.json(await options.service.list(userId));
  }));

  app.get("/orders/:id", route(async (request, response) => {
    response.json(await options.service.get(uuid.parse(request.params.id), principal(request)));
  }));

  app.patch("/orders/:id/cancel", route(async (request, response) => {
    response.json(await options.service.cancel(uuid.parse(request.params.id), principal(request)));
  }));

  app.use((error: unknown, _request: Request, response: Response, _next: NextFunction) => {
    if (error instanceof AuthenticationError) return response.status(401).json({ error: "authentication_required" });
    if (error instanceof AuthorizationError) return response.status(403).json({ error: "forbidden" });
    if (error instanceof OrderNotFoundError) return response.status(404).json({ error: "order_not_found" });
    if (error instanceof InvalidOrderTransitionError) return response.status(409).json({ error: "invalid_order_transition", message: error.message });
    if (error instanceof ValidationError || error instanceof z.ZodError) return response.status(400).json({ error: "invalid_request" });
    options.logger.error({ error }, "Unhandled request error");
    return response.status(500).json({ error: "internal_server_error" });
  });
  return app;
}

class AuthenticationError extends Error {}
class AuthorizationError extends Error {}
class ValidationError extends Error {}
function route(handler: (request: Request, response: Response) => Promise<void>): express.RequestHandler {
  return (request, response, next) => void handler(request, response).catch(next);
}
