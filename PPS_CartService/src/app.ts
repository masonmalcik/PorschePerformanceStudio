import cors from "cors";
import express, { type NextFunction, type Request, type Response } from "express";
import helmet from "helmet";
import { pinoHttp } from "pino-http";
import { z } from "zod";
import type { AppLogger } from "./ports.js";
import type { CartServicePort } from "./services/cart-service.js";

const idSchema = z.string().trim().min(1).max(128).regex(/^[A-Za-z0-9._:@-]+$/);
const itemSchema = z.object({
  itemId: idSchema,
  quantity: z.number().int().min(1).max(999),
  unitPrice: z.string().regex(/^\d{1,10}(?:\.\d{1,2})?$/),
}).strict();

export interface AppOptions {
  service: CartServicePort;
  logger: AppLogger;
  trustedUserHeader?: string;
  corsOrigin?: string;
}

export function createApp(options: AppOptions): express.Express {
  const app = express();
  const identityHeader = options.trustedUserHeader ?? "x-authenticated-user-id";
  app.disable("x-powered-by");
  app.use(helmet());
  app.use(cors({ origin: options.corsOrigin ?? "http://localhost:3000", credentials: true }));
  app.use(express.json({ limit: "32kb" }));
  app.use(pinoHttp({ logger: options.logger as never }));

  app.get("/health", (_request, response) => response.json({ status: "ok" }));

  const authorize = (request: Request, response: Response, next: NextFunction): void => {
    const authenticatedUser = request.header(identityHeader);
    if (!authenticatedUser) {
      response.status(401).json({ error: "authentication_required" });
      return;
    }
    if (authenticatedUser !== request.params.userId) {
      response.status(403).json({ error: "forbidden" });
      return;
    }
    next();
  };

  app.get("/cart/:userId", authorize, asyncRoute(async (request, response) => {
    const userId = idSchema.parse(request.params.userId);
    response.json(await options.service.getCart(userId));
  }));

  app.post("/cart/:userId/items", authorize, asyncRoute(async (request, response) => {
    const userId = idSchema.parse(request.params.userId);
    const input = itemSchema.parse(request.body);
    response.status(200).json(await options.service.upsertItem(userId, input));
  }));

  app.delete("/cart/:userId/items/:itemId", authorize, asyncRoute(async (request, response) => {
    const userId = idSchema.parse(request.params.userId);
    const itemId = idSchema.parse(request.params.itemId);
    response.json(await options.service.removeItem(userId, itemId));
  }));

  app.delete("/cart/:userId", authorize, asyncRoute(async (request, response) => {
    const userId = idSchema.parse(request.params.userId);
    await options.service.clearCart(userId);
    response.status(204).end();
  }));

  app.use((error: unknown, _request: Request, response: Response, _next: NextFunction) => {
    if (error instanceof z.ZodError) {
      response.status(400).json({ error: "invalid_request", details: error.issues });
      return;
    }
    options.logger.error({ error }, "Unhandled request error");
    response.status(500).json({ error: "internal_server_error" });
  });
  return app;
}

function asyncRoute(
  handler: (request: Request, response: Response, next: NextFunction) => Promise<void>,
): (request: Request, response: Response, next: NextFunction) => void {
  return (request, response, next) => void handler(request, response, next).catch(next);
}
