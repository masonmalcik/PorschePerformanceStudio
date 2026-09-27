import { z } from "zod";

const schema = z.object({
  DATABASE_URL: z.string().min(1),
  STRIPE_SECRET_KEY: z.string().regex(/^sk_test_/, "Stripe test-mode secret key is required for the PPS demo"),
  STRIPE_WEBHOOK_SECRET: z.string().min(1),
  PORT: z.coerce.number().int().min(1).max(65_535).default(3006),
  LOG_LEVEL: z.string().default("info"),
  TRUSTED_USER_HEADER: z.string().regex(/^[a-z0-9-]+$/).default("x-authenticated-user-id"),
  CORS_ORIGIN: z.string().default("http://localhost:3000"),
});
export type Config = z.infer<typeof schema>;
export const loadConfig = (environment: NodeJS.ProcessEnv = process.env): Config => schema.parse(environment);
