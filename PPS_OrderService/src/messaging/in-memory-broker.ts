import type { DomainEvent } from "../domain/order.js";
import type { MessageConsumer, MessageHandler, MessagePublisher } from "../ports.js";

export class InMemoryBroker implements MessagePublisher, MessageConsumer {
  private readonly handlers = new Map<string, Set<MessageHandler>>();

  public async publish(event: DomainEvent): Promise<void> {
    for (const handler of this.handlers.get(event.type) ?? []) await handler(event);
  }

  public async subscribe(eventType: string, handler: MessageHandler): Promise<() => Promise<void>> {
    const handlers = this.handlers.get(eventType) ?? new Set<MessageHandler>();
    handlers.add(handler);
    this.handlers.set(eventType, handlers);
    return async () => { handlers.delete(handler); };
  }
}
