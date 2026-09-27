import { useEffect, useState } from 'react';
import { completeAuthentication } from '../lib/auth';
import { appPath } from '../lib/navigation';

export default function AuthCallback() {
  const [error, setError] = useState('');
  useEffect(() => {
    const params = new URLSearchParams(location.search);
    const code = params.get('code'); const state = params.get('state'); const providerError = params.get('error_description') ?? params.get('error');
    if (providerError) { setError(providerError); return; }
    if (!code || !state) { setError('Cognito did not return a valid authorization response.'); return; }
    completeAuthentication(code, state).then((returnTo) => location.replace(returnTo)).catch((reason: unknown) => setError(reason instanceof Error ? reason.message : 'Sign-in could not be completed.'));
  }, []);
  return <div className="auth-card" role="status">{error ? <><h2>Sign-in was not completed.</h2><p>{error}</p><a className="primary-action" href={appPath('/signin')}>Try again <span>→</span></a></> : <><div className="auth-spinner"/><h2>Completing secure sign-in…</h2><p>Please keep this page open.</p></>}</div>;
}
