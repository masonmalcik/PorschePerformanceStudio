import { useEffect, useState } from 'react';
import { beginAuthentication, getSession } from '../lib/auth';
import { appPath } from '../lib/navigation';

export default function AuthPanel() {
  const [signedIn, setSignedIn] = useState(false);
  useEffect(() => setSignedIn(Boolean(getSession())), []);
  const returnTo = typeof window === 'undefined'
    ? appPath('/account')
    : new URLSearchParams(window.location.search).get('returnTo') || appPath('/account');
  if (signedIn) return <div className="auth-card"><h2>You are signed in.</h2><p>Continue to your account, cart, or the performance catalog.</p><a className="primary-action" href={appPath('/account')}>Open account <span>→</span></a></div>;
  return <div className="auth-card">
    <p className="form-kicker">SECURE COGNITO ACCESS</p><h2>Welcome to the Studio.</h2>
    <p>Sign in using the secure PPS identity service. Your password is handled by AWS Cognito and is never sent to this website.</p>
    <button className="primary-action" onClick={() => void beginAuthentication('login', returnTo)}>Sign in <span>→</span></button>
    <button className="secondary-action" onClick={() => void beginAuthentication('signup', returnTo)}>Create an account</button>
    <button className="text-action" onClick={() => void beginAuthentication('recovery', returnTo)}>Forgot or reset password</button>
    <p className="auth-note">New accounts confirm their email through Cognito before signing in.</p>
  </div>;
}
