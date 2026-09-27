import memjs from "memjs";
import type { CartCache } from "../ports.js";

export class MemcachedCartCache implements CartCache {
  private readonly client: memjs.Client;

  public constructor(servers: string, username?: string, password?: string) {
    this.client = memjs.Client.create(servers, {
      username,
      password,
      timeout: 0.5,
      retries: 1,
      failover: true,
    });
  }

  public async get(key: string): Promise<string | null> {
    const result = await this.client.get(key);
    return result.value?.toString("utf8") ?? null;
  }

  public async set(key: string, value: string, ttlSeconds: number): Promise<void> {
    const stored = await this.client.set(key, Buffer.from(value), { expires: ttlSeconds });
    if (!stored) throw new Error(`Memcached rejected key ${key}`);
  }

  public async delete(key: string): Promise<void> {
    await this.client.delete(key);
  }

  public close(): void {
    this.client.close();
  }
}
