import { z } from "zod";

const schema = z.object({
  NODE_ENV: z.enum(["development", "test", "production"]).default("development"),
  DATABASE_URL: z.string().min(1),
  PORT: z.coerce.number().int().min(1).max(65_535).default(3005),
  LOG_LEVEL: z.string().default("info"),
  TRUSTED_USER_HEADER: z.string().regex(/^[a-z0-9-]+$/).default("x-authenticated-user-id"),
  CORS_ORIGIN: z.string().default("http://localhost:3000"),
  OUTBOX_POLL_INTERVAL_MS: z.coerce.number().int().min(100).default(1000),
  OUTBOX_BATCH_SIZE: z.coerce.number().int().min(1).max(500).default(50),
});
export type Config = z.infer<typeof schema>;
export const loadConfig = (environment: NodeJS.ProcessEnv = process.env): Config => schema.parse(environment);
