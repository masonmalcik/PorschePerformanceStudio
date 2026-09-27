import { CART_TTL_SECONDS, type Cart, type UpsertCartItem } from "../domain/cart.js";
import type { AppLogger, CartCache, CartDatabase, CartRepositoryPort } from "../ports.js";

export class CartRepository implements CartRepositoryPort {
  public constructor(
    private readonly database: CartDatabase,
    private readonly cache: CartCache,
    private readonly logger: AppLogger,
    private readonly ttlSeconds = CART_TTL_SECONDS,
  ) {}

  public async get(userId: string): Promise<Cart | null> {
    const key = this.key(userId);
    try {
      const cached = await this.cache.get(key);
      if (cached !== null) {
        try {
          return JSON.parse(cached) as Cart;
        } catch (error) {
          this.logger.warn({ error, key }, "Invalid cached cart; falling back to DynamoDB");
          await this.tryDelete(key);
        }
      }
    } catch (error) {
      this.logger.warn({ error, key }, "Memcached read failed; falling back to DynamoDB");
    }

    const cart = await this.database.findByUserId(userId);
    if (cart !== null) await this.trySet(key, cart);
    return cart;
  }

  public async upsertItem(userId: string, input: UpsertCartItem): Promise<Cart> {
    const cart = await this.database.upsertItem(userId, input, this.nextExpiry());
    await this.trySet(this.key(userId), cart);
    return cart;
  }

  public async removeItem(userId: string, itemId: string): Promise<Cart | null> {
    const cart = await this.database.removeItem(userId, itemId, this.nextExpiry());
    if (cart === null) await this.tryDelete(this.key(userId));
    else await this.trySet(this.key(userId), cart);
    return cart;
  }

  public async clear(userId: string): Promise<void> {
    await this.database.deleteByUserId(userId);
    await this.tryDelete(this.key(userId));
  }

  private key(userId: string): string {
    return `cart:${userId}`;
  }

  private nextExpiry(): Date {
    return new Date(Date.now() + this.ttlSeconds * 1000);
  }

  private async trySet(key: string, cart: Cart): Promise<void> {
    try {
      await this.cache.set(key, JSON.stringify(cart), this.ttlSeconds);
    } catch (error) {
      this.logger.warn({ error, key }, "Memcached write failed; DynamoDB remains authoritative");
    }
  }

  private async tryDelete(key: string): Promise<void> {
    try {
      await this.cache.delete(key);
    } catch (error) {
      this.logger.warn({ error, key }, "Memcached invalidation failed");
    }
  }
}
