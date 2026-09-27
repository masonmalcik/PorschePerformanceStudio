import { emptyCart, type Cart, type UpsertCartItem } from "../domain/cart.js";
import type { CartRepositoryPort } from "../ports.js";

export interface CartServicePort {
  getCart(userId: string): Promise<Cart>;
  upsertItem(userId: string, input: UpsertCartItem): Promise<Cart>;
  removeItem(userId: string, itemId: string): Promise<Cart>;
  clearCart(userId: string): Promise<void>;
}

export class CartService implements CartServicePort {
  public constructor(private readonly repository: CartRepositoryPort) {}

  public async getCart(userId: string): Promise<Cart> {
    return (await this.repository.get(userId)) ?? emptyCart(userId);
  }

  public upsertItem(userId: string, input: UpsertCartItem): Promise<Cart> {
    return this.repository.upsertItem(userId, input);
  }

  public async removeItem(userId: string, itemId: string): Promise<Cart> {
    return (await this.repository.removeItem(userId, itemId)) ?? emptyCart(userId);
  }

  public clearCart(userId: string): Promise<void> {
    return this.repository.clear(userId);
  }
}
