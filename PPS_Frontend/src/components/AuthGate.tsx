import { useEffect, useState } from 'react';
import { getSession, isAdmin } from '../lib/auth';
import { appPath } from '../lib/navigation';

export default function AuthGate({ admin = false }: { admin?: boolean }) {
  const [state, setState] = useState<'checking'|'allowed'|'denied'>('checking');
  useEffect(() => {
    const session = getSession();
    if (!session) { location.replace(`${appPath('/signin')}?returnTo=${encodeURIComponent(location.pathname + location.search)}`); return; }
    const allowed = !admin || isAdmin(); setState(allowed ? 'allowed' : 'denied');
    document.documentElement.dataset.authReady = allowed ? 'true' : 'denied';
  }, [admin]);
  if (state === 'denied') return <div className="route-blocker"><div><h1>Administrator access required.</h1><p>This account does not have the PPS administrator group and Catalog scope.</p><a href={appPath('/account')}>Return to account</a></div></div>;
  return null;
}
