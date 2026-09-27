import { getSession } from './auth';
import { cartApiBase } from './api';
import { appPath } from './navigation';
export async function addCartItem(itemId: string, unitPrice: string, quantity = 1): Promise<void> {
  const session = getSession();
  if (!session) { location.assign(`${appPath('/signin')}?returnTo=${encodeURIComponent(location.pathname)}`); return; }
  const response = await fetch(`${cartApiBase}/${encodeURIComponent(session.subject)}/items`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ itemId, unitPrice, quantity }) });
  if (!response.ok) throw new Error('The product could not be added to your cart.');
  window.dispatchEvent(new Event('pps:cart-changed'));
}
