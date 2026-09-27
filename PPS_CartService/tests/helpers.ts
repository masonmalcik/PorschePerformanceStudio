import type { Cart, UpsertCartItem } from "../src/domain/cart.js";
import type { AppLogger, CartCache, CartDatabase } from "../src/ports.js";
import pino from "pino";

export const silentLogger: AppLogger = pino({ level: "silent" });

export function cartFixture(userId = "user-1", items: Cart["items"] = []): Cart {
  return {
    id: `cart-${userId}`,
    userId,
    items,
    createdAt: "2026-01-01T00:00:00.000Z",
    updatedAt: "2026-01-01T00:00:00.000Z",
    expiresAt: "2026-01-02T00:00:00.000Z",
  };
}

export class FakeCache implements CartCache {
  public readonly values = new Map<string, string>();
  public gets = 0;
  public sets = 0;
  public deletes = 0;
  public unavailable = false;

  public async get(key: string): Promise<string | null> {
    this.gets++;
    if (this.unavailable) throw new Error("cache unavailable");
    return this.values.get(key) ?? null;
  }
  public async set(key: string, value: string): Promise<void> {
    this.sets++;
    if (this.unavailable) throw new Error("cache unavailable");
    this.values.set(key, value);
  }
  public async delete(key: string): Promise<void> {
    this.deletes++;
    if (this.unavailable) throw new Error("cache unavailable");
    this.values.delete(key);
  }
}

export class FakeDatabase implements CartDatabase {
  public readonly carts = new Map<string, Cart>();
  public reads = 0;
  public writes = 0;

  public async findByUserId(userId: string): Promise<Cart | null> {
    this.reads++;
    return structuredClone(this.carts.get(userId) ?? null);
  }
  public async upsertItem(userId: string, input: UpsertCartItem, expiresAt: Date): Promise<Cart> {
    this.writes++;
    const current = this.carts.get(userId) ?? cartFixture(userId);
    const now = new Date().toISOString();
    const item = { ...input, createdAt: now, updatedAt: now };
    const items = [...current.items.filter((value) => value.itemId !== input.itemId), item];
    const cart = { ...current, items, updatedAt: now, expiresAt: expiresAt.toISOString() };
    this.carts.set(userId, cart);
    return structuredClone(cart);
  }
  public async removeItem(userId: string, itemId: string, expiresAt: Date): Promise<Cart | null> {
    this.writes++;
    const current = this.carts.get(userId);
    if (!current) return null;
    const cart = { ...current, items: current.items.filter((item) => item.itemId !== itemId), expiresAt: expiresAt.toISOString() };
    this.carts.set(userId, cart);
    return structuredClone(cart);
  }
  public async deleteByUserId(userId: string): Promise<void> {
    this.writes++;
    this.carts.delete(userId);
  }
}
