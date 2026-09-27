import { randomUUID } from "node:crypto";
import {
  DeleteCommand,
  DynamoDBDocumentClient,
  GetCommand,
  PutCommand,
} from "@aws-sdk/lib-dynamodb";
import type { Cart, CartItem, UpsertCartItem } from "../domain/cart.js";
import type { CartDatabase } from "../ports.js";

interface CartDocument {
  userId: string;
  id: string;
  items: CartItem[];
  createdAt: string;
  updatedAt: string;
  expiresAt: string;
  ttl: number;
  version: number;
}

export class DynamoCartDatabase implements CartDatabase {
  public constructor(
    private readonly client: DynamoDBDocumentClient,
    private readonly tableName: string,
    private readonly maxWriteAttempts = 4,
  ) {}

  public async findByUserId(userId: string): Promise<Cart | null> {
    const document = await this.load(userId);
    return document === null ? null : this.toCart(document);
  }

  public async upsertItem(userId: string, input: UpsertCartItem, expiresAt: Date): Promise<Cart> {
    const cart = await this.mutate(userId, expiresAt, true, (items, now) => {
      const existing = items.find((item) => item.itemId === input.itemId);
      const next: CartItem = {
        ...input,
        createdAt: existing?.createdAt ?? now,
        updatedAt: now,
      };
      return [...items.filter((item) => item.itemId !== input.itemId), next];
    });
    if (cart === null) throw new Error("Unable to create cart");
    return cart;
  }

  public async removeItem(userId: string, itemId: string, expiresAt: Date): Promise<Cart | null> {
    return this.mutate(
      userId,
      expiresAt,
      false,
      (items) => items.filter((item) => item.itemId !== itemId),
    );
  }

  public async deleteByUserId(userId: string): Promise<void> {
    await this.client.send(new DeleteCommand({ TableName: this.tableName, Key: { userId } }));
  }

  private async mutate(
    userId: string,
    expiresAt: Date,
    createIfMissing: boolean,
    updateItems: (items: CartItem[], now: string) => CartItem[],
  ): Promise<Cart | null> {
    for (let attempt = 1; attempt <= this.maxWriteAttempts; attempt++) {
      const existing = await this.load(userId);
      if (existing === null && !createIfMissing) return null;
      const now = new Date().toISOString();
      const document: CartDocument = {
        userId,
        id: existing?.id ?? randomUUID(),
        items: updateItems(existing?.items ?? [], now),
        createdAt: existing?.createdAt ?? now,
        updatedAt: now,
        expiresAt: expiresAt.toISOString(),
        ttl: Math.floor(expiresAt.getTime() / 1000),
        version: (existing?.version ?? 0) + 1,
      };
      try {
        await this.client.send(new PutCommand({
          TableName: this.tableName,
          Item: document,
          ConditionExpression: existing === null
            ? "attribute_not_exists(userId)"
            : "#version = :expectedVersion",
          ExpressionAttributeNames: existing === null ? undefined : { "#version": "version" },
          ExpressionAttributeValues: existing === null ? undefined : { ":expectedVersion": existing.version },
        }));
        return this.toCart(document);
      } catch (error) {
        if (!this.isConcurrentWrite(error) || attempt === this.maxWriteAttempts) throw error;
      }
    }
    throw new Error("Cart update retry budget exhausted");
  }

  private async load(userId: string): Promise<CartDocument | null> {
    const response = await this.client.send(new GetCommand({
      TableName: this.tableName,
      Key: { userId },
      ConsistentRead: true,
    }));
    const document = (response.Item as CartDocument | undefined) ?? null;
    if (document !== null && document.ttl <= Math.floor(Date.now() / 1000)) {
      try {
        await this.client.send(new DeleteCommand({
          TableName: this.tableName,
          Key: { userId },
          ConditionExpression: "#ttl = :observedTtl",
          ExpressionAttributeNames: { "#ttl": "ttl" },
          ExpressionAttributeValues: { ":observedTtl": document.ttl },
        }));
      } catch (error) {
        if (!this.isConcurrentWrite(error)) throw error;
      }
      return null;
    }
    return document;
  }

  private isConcurrentWrite(error: unknown): boolean {
    return error instanceof Error && error.name === "ConditionalCheckFailedException";
  }

  private toCart(document: CartDocument): Cart {
    return {
      id: document.id,
      userId: document.userId,
      items: document.items,
      createdAt: document.createdAt,
      updatedAt: document.updatedAt,
      expiresAt: document.expiresAt,
    };
  }
}
