import type { AppLogger, MessagePublisher, OutboxStore } from "../ports.js";

export class OutboxWorker {
  private timer: NodeJS.Timeout | undefined;
  private running = false;

  public constructor(
    private readonly store: OutboxStore,
    private readonly publisher: MessagePublisher,
    private readonly logger: AppLogger,
    private readonly intervalMs: number,
    private readonly batchSize: number,
  ) {}

  public start(): void {
    if (this.timer) return;
    this.timer = setInterval(() => void this.runOnce(), this.intervalMs);
    this.timer.unref();
  }

  public stop(): void {
    if (this.timer) clearInterval(this.timer);
    this.timer = undefined;
  }

  public async runOnce(): Promise<void> {
    if (this.running) return;
    this.running = true;
    try {
      for (const event of await this.store.claimBatch(this.batchSize)) {
        try {
          await this.publisher.publish(event);
          await this.store.markPublished(event.eventId);
        } catch (error) {
          this.logger.warn({ error, eventId: event.eventId }, "Outbox publish failed; scheduling retry");
          await this.store.markFailed(event.eventId, new Date(Date.now() + 5_000));
        }
      }
    } finally {
      this.running = false;
    }
  }
}
