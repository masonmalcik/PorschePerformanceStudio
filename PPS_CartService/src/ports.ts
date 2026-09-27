import type { Cart, UpsertCartItem } from "./domain/cart.js";

export interface CartDatabase {
  findByUserId(userId: string): Promise<Cart | null>;
  upsertItem(userId: string, input: UpsertCartItem, expiresAt: Date): Promise<Cart>;
  removeItem(userId: string, itemId: string, expiresAt: Date): Promise<Cart | null>;
  deleteByUserId(userId: string): Promise<void>;
}

export interface CartCache {
  get(key: string): Promise<string | null>;
  set(key: string, value: string, ttlSeconds: number): Promise<void>;
  delete(key: string): Promise<void>;
}

export interface CartRepositoryPort {
  get(userId: string): Promise<Cart | null>;
  upsertItem(userId: string, input: UpsertCartItem): Promise<Cart>;
  removeItem(userId: string, itemId: string): Promise<Cart | null>;
  clear(userId: string): Promise<void>;
}

export interface AppLogger {
  warn(context: object, message: string): void;
  error(context: object, message: string): void;
  info(context: object, message: string): void;
}
