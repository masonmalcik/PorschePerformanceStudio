import { useEffect, useState } from 'react';
import { authenticatedFetch, authApiBase, getSession, isAdmin, signOut } from '../lib/auth';
import { orderApiBase } from '../lib/api';
import { appPath } from '../lib/navigation';
type ProfileResponse = { identity: { sub: string; email: string; email_verified: boolean; username: string }; profile: { displayName?: string; locale?: string; timezone?: string } };
type Order = { id: string; status: string; totalAmount: string; createdAt: string };
export default function AccountProfile() {
  const [profile, setProfile] = useState<ProfileResponse | null>(null); const [orders, setOrders] = useState<Order[]>([]); const [error, setError] = useState('');
  useEffect(() => { const session=getSession(); if(!session)return; Promise.all([authenticatedFetch(`${authApiBase}/me`),authenticatedFetch(`${orderApiBase}/user/${encodeURIComponent(session.subject)}`)]).then(async([me,history])=>{if(!me.ok)throw Error('Your profile could not be loaded.');setProfile(await me.json());if(history.ok)setOrders(await history.json())}).catch(e=>setError(e.message)); }, []);
  if (error) return <div className="empty-state"><h2>Account unavailable</h2><p>{error}</p></div>;
  if (!profile) return <div className="empty-state"><h2>Loading account…</h2></div>;
  return <div className="account-grid"><section className="account-card"><p className="form-kicker">IDENTITY</p><h2>{profile.profile.displayName || profile.identity.username || 'PPS Member'}</h2><dl><div><dt>Email</dt><dd>{profile.identity.email}</dd></div><div><dt>Email status</dt><dd>{profile.identity.email_verified?'Verified':'Unverified'}</dd></div><div><dt>Access</dt><dd>{isAdmin()?'Administrator':'Customer'}</dd></div></dl><div className="account-actions"><a href={appPath('/cart')}>View cart</a>{isAdmin()&&<a href={appPath('/admin/products/new')}>Admin tools</a>}<button onClick={signOut}>Sign out</button></div></section><section className="account-card"><p className="form-kicker">ORDER HISTORY</p><h2>Your orders</h2>{orders.length?<div className="order-list">{orders.map(order=><a href={`${appPath('/order')}?id=${encodeURIComponent(order.id)}`} key={order.id}><span>{new Date(order.createdAt).toLocaleDateString()}</span><strong>{order.status.replaceAll('_',' ')}</strong><span>${Number(order.totalAmount).toFixed(2)}</span></a>)}</div>:<p>No orders yet. Your completed checkouts will appear here.</p>}</section></div>;
}
