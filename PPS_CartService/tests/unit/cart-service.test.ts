import { describe, expect, it, vi } from "vitest";
import type { CartRepositoryPort } from "../../src/ports.js";
import { CartService } from "../../src/services/cart-service.js";
import { cartFixture } from "../helpers.js";

function repositoryMock(): CartRepositoryPort {
  return {
    get: vi.fn(),
    upsertItem: vi.fn(),
    removeItem: vi.fn(),
    clear: vi.fn(),
  };
}

describe("CartService", () => {
  it("returns an empty cart when no durable cart exists", async () => {
    const repository = repositoryMock();
    vi.mocked(repository.get).mockResolvedValue(null);
    const result = await new CartService(repository).getCart("user-1");
    expect(result.userId).toBe("user-1");
    expect(result.items).toEqual([]);
  });

  it("delegates a validated item update", async () => {
    const repository = repositoryMock();
    const expected = cartFixture("user-1");
    vi.mocked(repository.upsertItem).mockResolvedValue(expected);
    const input = { itemId: "sku-1", quantity: 2, unitPrice: "129.99" };
    await expect(new CartService(repository).upsertItem("user-1", input)).resolves.toEqual(expected);
    expect(repository.upsertItem).toHaveBeenCalledWith("user-1", input);
  });

  it("clears the cart during checkout", async () => {
    const repository = repositoryMock();
    vi.mocked(repository.clear).mockResolvedValue(undefined);
    await new CartService(repository).clearCart("user-1");
    expect(repository.clear).toHaveBeenCalledWith("user-1");
  });
});
