import { useEffect, useState } from 'react';
import { beginAuthentication, getSession, onAuthChange, signOut, type AuthSession } from '../lib/auth';
import { appPath } from '../lib/navigation';

export default function AuthControls() {
  const [session, setSession] = useState<AuthSession | null>(null);
  useEffect(() => { const update = () => setSession(getSession()); update(); return onAuthChange(update); }, []);
  if (!session) return <button className="nav-auth" type="button" onClick={() => void beginAuthentication('login', location.pathname)}>Sign in</button>;
  return <div className="nav-session"><a href={appPath('/account')}>{session.username || session.email || 'Account'}</a><button type="button" onClick={signOut}>Sign out</button></div>;
}
