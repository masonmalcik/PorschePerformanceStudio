import { describe, expect, it, vi } from "vitest";
import { CartRepository } from "../../src/repositories/cart-repository.js";
import { cartFixture, FakeCache, FakeDatabase, silentLogger } from "../helpers.js";

describe("CartRepository cache-aside integration", () => {
  it("backfills a miss and serves the next read without touching DynamoDB", async () => {
    const database = new FakeDatabase();
    const cache = new FakeCache();
    database.carts.set("user-1", cartFixture());
    const repository = new CartRepository(database, cache, silentLogger);
    await expect(repository.get("user-1")).resolves.toEqual(cartFixture());
    expect(database.reads).toBe(1);
    expect(cache.values.has("cart:user-1")).toBe(true);
    await repository.get("user-1");
    expect(database.reads).toBe(1);
    expect(cache.gets).toBe(2);
  });

  it("writes to DynamoDB before publishing the updated cache value", async () => {
    const database = new FakeDatabase();
    const cache = new FakeCache();
    const repository = new CartRepository(database, cache, silentLogger);
    const cart = await repository.upsertItem("user-1", { itemId: "sku-1", quantity: 3, unitPrice: "25.00" });
    expect(database.writes).toBe(1);
    expect(cart.items[0]?.quantity).toBe(3);
    const cached = await repository.get("user-1");
    expect(cached?.items[0]?.quantity).toBe(3);
    expect(database.reads).toBe(0);
  });

  it("invalidates cache when clearing a cart", async () => {
    const database = new FakeDatabase();
    const cache = new FakeCache();
    database.carts.set("user-1", cartFixture());
    cache.values.set("cart:user-1", JSON.stringify(cartFixture()));
    await new CartRepository(database, cache, silentLogger).clear("user-1");
    expect(database.carts.has("user-1")).toBe(false);
    expect(cache.values.has("cart:user-1")).toBe(false);
    expect(cache.deletes).toBe(1);
  });

  it("fails open to DynamoDB when Memcached is unavailable", async () => {
    const database = new FakeDatabase();
    const cache = new FakeCache();
    const logger = { ...silentLogger, warn: vi.fn() };
    database.carts.set("user-1", cartFixture());
    cache.unavailable = true;
    await expect(new CartRepository(database, cache, logger).get("user-1")).resolves.toEqual(cartFixture());
    expect(database.reads).toBe(1);
    expect(logger.warn).toHaveBeenCalled();
  });
});
