import { describe, expect, it, vi } from "vitest";
import type { AppLogger, MessagePublisher, OutboxStore } from "../../src/ports.js";
import { OutboxWorker } from "../../src/messaging/outbox-worker.js";

const logger: AppLogger = { info: vi.fn(), warn: vi.fn(), error: vi.fn() };
const event = { eventId: "11111111-1111-4111-8111-111111111111", type: "OrderCreated", aggregateId: "22222222-2222-4222-8222-222222222222", occurredAt: new Date().toISOString(), data: {} };

describe("OutboxWorker", () => {
  it("publishes and marks an event", async () => {
    const store: OutboxStore = { claimBatch: vi.fn().mockResolvedValue([event]), markPublished: vi.fn(), markFailed: vi.fn() };
    const publisher: MessagePublisher = { publish: vi.fn() };
    await new OutboxWorker(store, publisher, logger, 1000, 10).runOnce();
    expect(publisher.publish).toHaveBeenCalledWith(event);
    expect(store.markPublished).toHaveBeenCalledWith(event.eventId);
  });

  it("schedules a retry when the broker is unavailable", async () => {
    const store: OutboxStore = { claimBatch: vi.fn().mockResolvedValue([event]), markPublished: vi.fn(), markFailed: vi.fn() };
    const publisher: MessagePublisher = { publish: vi.fn().mockRejectedValue(new Error("broker down")) };
    await new OutboxWorker(store, publisher, logger, 1000, 10).runOnce();
    expect(store.markPublished).not.toHaveBeenCalled();
    expect(store.markFailed).toHaveBeenCalledWith(event.eventId, expect.any(Date));
  });
});
