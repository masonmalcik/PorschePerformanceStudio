import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";

export default function setup(): void {
  const testDatabaseUrl = process.env.TEST_DATABASE_URL;
  if (!testDatabaseUrl) return;
  const applicationDatabaseUrl = process.env.DATABASE_URL ?? envFileValue("DATABASE_URL");
  if (testDatabaseUrl === applicationDatabaseUrl) {
    throw new Error("TEST_DATABASE_URL must not equal DATABASE_URL");
  }
  execFileSync(process.platform === "win32" ? "npx.cmd" : "npx", ["prisma", "migrate", "deploy"], {
    cwd: process.cwd(),
    env: { ...process.env, DATABASE_URL: testDatabaseUrl },
    stdio: "inherit",
  });
}

function envFileValue(name: string): string | undefined {
  if (!existsSync(".env")) return undefined;
  const line = readFileSync(".env", "utf8").split(/\r?\n/).find((entry) => entry.startsWith(`${name}=`));
  return line?.slice(name.length + 1).trim().replace(/^['"]|['"]$/g, "");
}
