import pino from "pino";
import request from "supertest";
import { describe, expect, it, vi } from "vitest";
import { createApp } from "../../src/app.js";
import type { OrderRepository } from "../../src/ports.js";
import { OrderService } from "../../src/services/order-service.js";
import { orderFixture } from "../helpers.js";

const logger = pino({ level: "silent" });
function setup() {
  const repository: OrderRepository = {
    create: vi.fn(), findById: vi.fn(), findByUserId: vi.fn(), transition: vi.fn(),
  };
  return { repository, app: createApp({ service: new OrderService(repository), logger }) };
}

describe("order HTTP API", () => {
  it("requires an authenticated principal and idempotency key", async () => {
    const { app } = setup();
    const body = { items: [{ productId: "product-1", quantity: 1, pricePerUnit: "10.00" }] };
    expect((await request(app).post("/orders").send(body)).status).toBe(401);
    expect((await request(app).post("/orders").set("x-authenticated-user-id", "user-1").send(body)).status).toBe(400);
  });

  it("creates an order and reports whether it is an idempotent replay", async () => {
    const { app, repository } = setup();
    vi.mocked(repository.create).mockResolvedValue({ order: orderFixture(), replayed: false });
    const response = await request(app).post("/orders")
      .set("x-authenticated-user-id", "user-1")
      .set("idempotency_key", "checkout-1")
      .send({ items: [{ productId: "product-1", quantity: 2, pricePerUnit: "10.00" }] });
    expect(response.status).toBe(201);
    expect(response.headers["idempotency-replayed"]).toBe("false");
  });

  it("prevents users from listing another user's orders", async () => {
    const { app } = setup();
    const response = await request(app).get("/orders/user/user-2").set("x-authenticated-user-id", "user-1");
    expect(response.status).toBe(403);
  });
});
