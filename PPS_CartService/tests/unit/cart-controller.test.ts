import request from "supertest";
import { describe, expect, it, vi } from "vitest";
import { createApp } from "../../src/app.js";
import type { CartServicePort } from "../../src/services/cart-service.js";
import { cartFixture, silentLogger } from "../helpers.js";

function serviceMock(): CartServicePort {
  return { getCart: vi.fn(), upsertItem: vi.fn(), removeItem: vi.fn(), clearCart: vi.fn() };
}

describe("cart HTTP API", () => {
  it("returns a cart for the authenticated owner", async () => {
    const service = serviceMock();
    vi.mocked(service.getCart).mockResolvedValue(cartFixture());
    const response = await request(createApp({ service, logger: silentLogger }))
      .get("/cart/user-1").set("x-authenticated-user-id", "user-1");
    expect(response.status).toBe(200);
    expect(response.body.userId).toBe("user-1");
  });

  it("rejects missing or mismatched trusted identity", async () => {
    const app = createApp({ service: serviceMock(), logger: silentLogger });
    expect((await request(app).get("/cart/user-1")).status).toBe(401);
    expect((await request(app).get("/cart/user-1").set("x-authenticated-user-id", "user-2")).status).toBe(403);
  });

  it("validates item quantities and decimal money strings", async () => {
    const service = serviceMock();
    const response = await request(createApp({ service, logger: silentLogger }))
      .post("/cart/user-1/items").set("x-authenticated-user-id", "user-1")
      .send({ itemId: "sku-1", quantity: 0, unitPrice: 12.999 });
    expect(response.status).toBe(400);
    expect(service.upsertItem).not.toHaveBeenCalled();
  });

  it("clears a cart and returns no content", async () => {
    const service = serviceMock();
    vi.mocked(service.clearCart).mockResolvedValue(undefined);
    const response = await request(createApp({ service, logger: silentLogger }))
      .delete("/cart/user-1").set("x-authenticated-user-id", "user-1");
    expect(response.status).toBe(204);
  });
});
