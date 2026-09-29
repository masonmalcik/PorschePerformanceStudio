import { useEffect, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import AdminSelect from './AdminSelect';
import { catalogAdminApiBase, developmentAdminKey } from '../lib/api';

const endpoint = `${catalogAdminApiBase}/vehicle-generations`;
const adminKey = developmentAdminKey;

type Notice = { key: number; kind: 'error' | 'success'; title: string; message: string };
type CreatedGeneration = { name: string };
type VehicleModelOption = { id: string; name: string; modelCode: string };

export default function CreateVehicleGenerationForm() {
  const [vehicleModelId, setVehicleModelId] = useState('');
  const [vehicleModels, setVehicleModels] = useState<VehicleModelOption[]>([]);
  const [isLoadingModels, setIsLoadingModels] = useState(true);
  const [name, setName] = useState('');
  const [generationCode, setGenerationCode] = useState('');
  const [startYear, setStartYear] = useState('');
  const [endYear, setEndYear] = useState('');
  const [notices, setNotices] = useState<Notice[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [validationAttempted, setValidationAttempted] = useState(false);
  const nextKey = useRef(1);

  function notify(kind: Notice['kind'], title: string, message: string) {
    setNotices((current) => [{ key: nextKey.current++, kind, title, message }, ...current]);
  }

  useEffect(() => {
    let active = true;
    async function loadVehicleModels() {
      try {
        const response = await fetch(`${catalogAdminApiBase}/vehicle-models`, {
          headers: { 'x-pps-admin-key': adminKey },
        });
        if (!response.ok) throw new Error(`Unable to load vehicle models (${response.status}).`);
        const models = await response.json() as VehicleModelOption[];
        if (active) setVehicleModels(models);
      } catch (error) {
        if (active) notify('error', 'Models unavailable', error instanceof Error ? error.message : 'Unable to load vehicle models.');
      } finally {
        if (active) setIsLoadingModels(false);
      }
    }
    loadVehicleModels();
    return () => { active = false; };
  }, []);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    if (event.defaultPrevented) return;
    event.preventDefault();
    const modelId = vehicleModelId.trim();
    const generationName = name.trim();
    const code = generationCode.trim();
    if (!modelId || !generationName || !code || !startYear) {
      notify('error', 'Request failed', 'Enter a vehicle model ID, generation name, generation code, and start year.');
      return;
    }

    setIsSubmitting(true);
    try {
      const response = await fetch(endpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'x-pps-admin-key': adminKey },
        body: JSON.stringify({
          vehicleModelId: modelId,
          name: generationName,
          generationCode: code,
          timeframe: { startYear: Number(startYear), endYear: endYear ? Number(endYear) : null },
        }),
      });
      const raw = await response.text();
      let body: unknown = null;
      if (raw) { try { body = JSON.parse(raw); } catch { body = raw; } }
      if (!response.ok) {
        if (response.status === 409) throw new Error(`${generationName} already exists.`);
        const nested = typeof body === 'object' && body && 'error' in body ? body.error : null;
        const message = typeof nested === 'object' && nested && 'message' in nested
          ? String(nested.message)
          : `The API returned ${response.status} ${response.statusText}.`;
        throw new Error(message);
      }
      const created = body as CreatedGeneration;
      notify('success', 'Generation created', `${created.name} was created successfully.`);
      setVehicleModelId(''); setName(''); setGenerationCode(''); setStartYear(''); setEndYear('');
      setValidationAttempted(false);
    } catch (error) {
      notify('error', 'Request failed', error instanceof Error ? error.message : 'The request could not be completed.');
    } finally { setIsSubmitting(false); }
  }

  return <div className="admin-workspace">
    <form className="admin-form" noValidate onSubmitCapture={() => setValidationAttempted(true)} onSubmit={handleSubmit}>
      <div className="form-heading"><h2>Generation Details</h2></div>
      <label><span>Vehicle Model</span>
        <AdminSelect
          value={vehicleModelId}
          options={vehicleModels.map((model) => ({ value: model.id, label: model.name }))}
          placeholder={isLoadingModels ? 'Loading vehicle models…' : 'Select a vehicle model'}
          disabled={isLoadingModels}
          required
          onChange={setVehicleModelId}
        />
      </label>
      <label className={`form-field${validationAttempted&&!name.trim()?' invalid':''}`}><span>Generation Name</span><input autoComplete="off" value={name} onChange={(event) => setName(event.target.value)} />{validationAttempted&&!name.trim()&&<small className="field-error">This Field is Required</small>}</label>
      <label className={`form-field${validationAttempted&&!generationCode.trim()?' invalid':''}`}><span>Generation Code</span><input autoComplete="off" value={generationCode} onChange={(event) => setGenerationCode(event.target.value)} />{validationAttempted&&!generationCode.trim()&&<small className="field-error">This Field is Required</small>}</label>
      <label className={`form-field${validationAttempted&&!startYear?' invalid':''}`}><span>Start Year</span><input type="number" min="1900" max="2200" value={startYear} onChange={(event) => setStartYear(event.target.value)} />{validationAttempted&&!startYear&&<small className="field-error">This Field is Required</small>}</label>
      <label><span>End Year <small>(optional)</small></span><input type="number" min="1900" max="2200" value={endYear} onChange={(event) => setEndYear(event.target.value)} /></label>
      <button className="admin-submit" type="submit" disabled={isSubmitting}>{isSubmitting ? 'Creating generation…' : 'Create Generation'} <span aria-hidden="true">→</span></button>
    </form>
    {notices.length > 0 && <div className={`status-stack${notices.length > 10 ? ' scrollable' : ''}`} aria-label="Submission results" aria-live="polite">
      {notices.map((notice) => <div className={`status-card ${notice.kind}`} role={notice.kind === 'error' ? 'alert' : 'status'} key={notice.key}>
        <button className="status-dismiss" type="button" onClick={() => setNotices((current) => current.filter((item) => item.key !== notice.key))} aria-label={`Dismiss ${notice.title.toLowerCase()} notification`}>×</button>
        <strong>{notice.title}</strong><p>{notice.message}</p>
      </div>)}
    </div>}
  </div>;
}
