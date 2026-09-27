import { z } from "zod";

const environmentSchema = z.object({
  NODE_ENV: z.enum(["development", "test", "production"]).default("development"),
  PORT: z.coerce.number().int().min(1).max(65_535).default(3004),
  AWS_REGION: z.string().min(1).default("us-east-1"),
  DYNAMODB_CART_TABLE: z.string().min(3).default("pps-carts-dev"),
  DYNAMODB_ENDPOINT: z.string().url().optional(),
  MEMCACHED_SERVERS: z.string().min(1).default("localhost:11211"),
  MEMCACHED_USERNAME: z.string().optional(),
  MEMCACHED_PASSWORD: z.string().optional(),
  TRUSTED_USER_HEADER: z.string().regex(/^[a-z0-9-]+$/).default("x-authenticated-user-id"),
  CORS_ORIGIN: z.string().default("http://localhost:3000"),
  LOG_LEVEL: z.string().default("info"),
});

export type AppConfig = z.infer<typeof environmentSchema>;

export function loadConfig(environment: NodeJS.ProcessEnv = process.env): AppConfig {
  return environmentSchema.parse(environment);
}
