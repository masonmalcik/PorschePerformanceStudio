import { useEffect, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import AdminSelect from './AdminSelect';
import { catalogAdminApiBase, developmentAdminKey } from '../lib/api';

const api = catalogAdminApiBase;
const key = developmentAdminKey;

type Option = { id: string; name: string };
type GenerationOption = Option & { vehicleModelId: string };
type Notice = { key: number; kind: 'error' | 'success'; title: string; message: string };

export default function CreateVehicleTrimForm() {
  const [vehicleModelId, setVehicleModelId] = useState('');
  const [generationId, setGenerationId] = useState('');
  const [name, setName] = useState('');
  const [trimCode, setTrimCode] = useState('');
  const [startYear, setStartYear] = useState('');
  const [endYear, setEndYear] = useState('');
  const [vehicleModels, setVehicleModels] = useState<Option[]>([]);
  const [generations, setGenerations] = useState<GenerationOption[]>([]);
  const [notices, setNotices] = useState<Notice[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const next = useRef(1);

  function notify(kind: Notice['kind'], title: string, message: string) {
    setNotices((current) => [{ key: next.current++, kind, title, message }, ...current]);
  }

  useEffect(() => {
    async function loadOptions() {
      try {
        const responses = await Promise.all(
          ['vehicle-models', 'vehicle-generations'].map((path) =>
            fetch(`${api}/${path}`, { headers: { 'x-pps-admin-key': key } }),
          ),
        );
        if (responses.some((response) => !response.ok)) {
          throw new Error('Unable to load vehicle models and generations.');
        }
        const [models, generations] = await Promise.all(responses.map((response) => response.json()));
        setVehicleModels(models);
        setGenerations(generations);
      } catch (error) {
        notify('error', 'Selections unavailable', error instanceof Error ? error.message : 'Unable to load selections.');
      }
    }
    loadOptions();
  }, []);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!vehicleModelId) {
      notify('error', 'Vehicle model required', 'Select a vehicle model.');
      return;
    }
    if (!generationId) {
      notify('error', 'Generation required', 'Select a vehicle generation.');
      return;
    }
    if (!generations.some((generation) => generation.id === generationId && generation.vehicleModelId === vehicleModelId)) {
      notify('error', 'Generation mismatch', 'Select a generation belonging to the selected vehicle model.');
      return;
    }
    setSubmitting(true);
    try {
      const response = await fetch(`${api}/vehicle-trims`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'x-pps-admin-key': key },
        body: JSON.stringify({
          generationId,
          vehicleModels: [vehicleModelId],
          name: name.trim(),
          trimCode: trimCode.trim(),
          timeframe: { startYear: Number(startYear), endYear: endYear ? Number(endYear) : null },
        }),
      });
      const body = await response.json();
      if (!response.ok) {
        const generationName = generations.find((generation) => generation.id === generationId)?.name ?? 'this generation';
        throw new Error(response.status === 409
          ? `A trim with code ${trimCode.trim().toUpperCase()} already exists in ${generationName}.`
          : body?.error?.message ?? 'The trim could not be created.');
      }
      notify('success', 'Trim created', `${body.name} was created successfully.`);
      setVehicleModelId('');
      setGenerationId('');
      setName('');
      setTrimCode('');
      setStartYear('');
      setEndYear('');
    } catch (error) {
      notify('error', 'Request failed', error instanceof Error ? error.message : 'The request could not be completed.');
    } finally {
      setSubmitting(false);
    }
  }

  return <div className="admin-workspace">
    <form className="admin-form" onSubmit={submit}>
      <div className="form-heading"><h2>Trim Details</h2></div>
      <label><span>Vehicle Model</span>
        <AdminSelect
          value={vehicleModelId}
          options={vehicleModels.map((model) => ({ value: model.id, label: model.name }))}
          placeholder="Select a vehicle model"
          onChange={(id) => { setVehicleModelId(id); setGenerationId(''); }}
        />
      </label>
      {vehicleModelId && <label><span>Vehicle Generation</span>
        <AdminSelect value={generationId} options={generations.filter((generation) => generation.vehicleModelId === vehicleModelId).map((generation) => ({ value: generation.id, label: generation.name }))} placeholder="Select a vehicle generation" onChange={setGenerationId} />
      </label>}
      <label><span>Trim Name</span><input value={name} onChange={(event) => setName(event.target.value)} required /></label>
      <label><span>Trim Code</span><input value={trimCode} onChange={(event) => setTrimCode(event.target.value)} required /></label>
      <label><span>Start Year</span><input type="number" min="1900" max="2200" value={startYear} onChange={(event) => setStartYear(event.target.value)} required /></label>
      <label><span>End Year <small>(optional)</small></span><input type="number" min="1900" max="2200" value={endYear} onChange={(event) => setEndYear(event.target.value)} /></label>
      <button className="admin-submit" disabled={submitting}>{submitting ? 'Creating trim…' : 'Create Trim'} <span>→</span></button>
    </form>
    {notices.length > 0 && <div className="status-stack" aria-live="polite">
      {notices.map((notice) => <div className={`status-card ${notice.kind}`} key={notice.key}>
        <button className="status-dismiss" type="button" onClick={() => setNotices((current) => current.filter((value) => value.key !== notice.key))}>×</button>
        <strong>{notice.title}</strong><p>{notice.message}</p>
      </div>)}
    </div>}
  </div>;
}
