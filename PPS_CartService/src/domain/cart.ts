export interface CartItem {
  itemId: string;
  quantity: number;
  unitPrice: string;
  createdAt: string;
  updatedAt: string;
}

export interface Cart {
  id: string;
  userId: string;
  items: CartItem[];
  createdAt: string;
  updatedAt: string;
  expiresAt: string;
}

export interface UpsertCartItem {
  itemId: string;
  quantity: number;
  unitPrice: string;
}

export const CART_TTL_SECONDS = 24 * 60 * 60;

export function emptyCart(userId: string): Cart {
  const now = new Date();
  return {
    id: "",
    userId,
    items: [],
    createdAt: now.toISOString(),
    updatedAt: now.toISOString(),
    expiresAt: new Date(now.getTime() + CART_TTL_SECONDS * 1000).toISOString(),
  };
}
