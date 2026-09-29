import { useEffect, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import AdminSelect from './AdminSelect';
import { catalogAdminApiBase, developmentAdminKey } from '../lib/api';

const api = catalogAdminApiBase;
const key = developmentAdminKey;
type Model = { id: string; name: string };
type Generation = Model & { vehicleModelId: string };
type Trim = Model & { generationId: string };
type Notice = { id: number; kind: 'error' | 'success'; message: string };

export default function CreateEngineForm() {
  const [models, setModels] = useState<Model[]>([]);
  const [generations, setGenerations] = useState<Generation[]>([]);
  const [trims, setTrims] = useState<Trim[]>([]);
  const [vehicleModel, setVehicleModel] = useState('');
  const [vehicleGeneration, setVehicleGeneration] = useState('');
  const [vehicleTrims, setVehicleTrims] = useState<string[]>([]);
  const [alloyMaterial, setAlloyMaterial] = useState('');
  const [factoryCode, setFactoryCode] = useState('');
  const [displacement, setDisplacement] = useState('');
  const [horsepower, setHorsepower] = useState('');
  const [rpm, setRpm] = useState('');
  const [layout, setLayout] = useState('');
  const [aspirationType, setAspirationType] = useState('');
  const [fuelDelivery, setFuelDelivery] = useState('');
  const [notices, setNotices] = useState<Notice[]>([]);
  const [busy, setBusy] = useState(false);
  const nextId = useRef(1);

  function notify(kind: Notice['kind'], message: string) {
    setNotices((current) => [{ id: nextId.current++, kind, message }, ...current]);
  }

  useEffect(() => {
    async function loadOptions() {
      try {
        const responses = await Promise.all(
          ['vehicle-models', 'vehicle-generations', 'vehicle-trims'].map((path) =>
            fetch(`${api}/${path}`, { headers: { 'x-pps-admin-key': key } }),
          ),
        );
        if (responses.some((response) => !response.ok)) throw new Error('Unable to load vehicle selections.');
        const [modelOptions, generationOptions, trimOptions] = await Promise.all(responses.map((response) => response.json()));
        setModels(modelOptions);
        setGenerations(generationOptions);
        setTrims(trimOptions);
      } catch (error) {
        notify('error', error instanceof Error ? error.message : 'Unable to load selections.');
      }
    }
    loadOptions();
  }, []);

  async function submit(event: FormEvent) {
    if (event.defaultPrevented) return;
    event.preventDefault();
    if (!vehicleModel || !vehicleGeneration || vehicleTrims.length === 0 || !layout || !aspirationType || !fuelDelivery) {
      notify('error', 'Select a model, generation, at least one trim, engine layout, aspiration type, and fuel delivery.');
      return;
    }
    const liters = Number(displacement);
    if (!Number.isFinite(liters) || liters <= 0 || liters > 20) {
      notify('error', 'Displacement must be greater than 0 and no more than 20 liters.');
      return;
    }
    const horsepowerValue = Number(horsepower);
    const rpmValue = Number(rpm);
    if (![horsepowerValue, rpmValue].every((value) => Number.isInteger(value) && value > 0 && value <= 2147483647)) {
      notify('error', 'Horsepower and RPM must be positive whole numbers.');
      return;
    }
    setBusy(true);
    try {
      const response = await fetch(`${api}/engines`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'x-pps-admin-key': key },
        body: JSON.stringify({
          vehicleModel, vehicleGeneration, vehicleTrims,
          alloyMaterial: alloyMaterial.trim(), factoryCode: factoryCode.trim(),
          displacement: liters, horsepower: horsepowerValue, rpm: rpmValue, layout, aspirationType, fuelDelivery,
        }),
      });
      const body = await response.json();
      if (!response.ok) throw new Error(body?.error?.message ?? 'The engine could not be created.');
      notify('success', `${body.factoryCode} was created successfully.`);
      setVehicleModel(''); setVehicleGeneration(''); setVehicleTrims([]);
      setAlloyMaterial(''); setFactoryCode(''); setDisplacement(''); setHorsepower(''); setRpm('');
      setLayout(''); setAspirationType(''); setFuelDelivery('');
    } catch (error) {
      notify('error', error instanceof Error ? error.message : 'Request failed.');
    } finally {
      setBusy(false);
    }
  }

  return <div className="admin-workspace">
    <form className="admin-form" onSubmit={submit}>
      <div className="form-heading"><h2>Engine Details</h2></div>
      <label><span>Vehicle Model</span><AdminSelect value={vehicleModel} options={models.map((model) => ({ value: model.id, label: model.name }))} placeholder="Select a vehicle model" required onChange={(id) => { setVehicleModel(id); setVehicleGeneration(''); setVehicleTrims([]); }} /></label>
      {vehicleModel && <label><span>Vehicle Generation</span><AdminSelect value={vehicleGeneration} options={generations.filter((generation) => generation.vehicleModelId === vehicleModel).map((generation) => ({ value: generation.id, label: generation.name }))} placeholder="Select a vehicle generation" required onChange={(id) => { setVehicleGeneration(id); setVehicleTrims([]); }} /></label>}
      {vehicleGeneration && <><label><span>Vehicle Trims</span><AdminSelect value={vehicleTrims[0]??''} options={trims.filter((trim) => trim.generationId === vehicleGeneration && !vehicleTrims.includes(trim.id)).map((trim) => ({ value: trim.id, label: trim.name }))} placeholder="Select vehicle trims" required onChange={(id) => setVehicleTrims((current) => [...current, id])} /></label>
        {vehicleTrims.length > 0 && <div className="admin-selection-tags" aria-label="Selected vehicle trims">{vehicleTrims.map((id) =>
          <span className="admin-selection-tag" key={id}>{trims.find((trim) => trim.id === id)?.name ?? id}
            <button type="button" aria-label={`Remove ${trims.find((trim) => trim.id === id)?.name ?? 'vehicle trim'}`} onClick={() => setVehicleTrims((current) => current.filter((value) => value !== id))}>×</button>
          </span>)}</div>}</>}
      <label><span>Engine Layout</span><AdminSelect value={layout} options={['F6', 'F4', 'I4', 'I5', 'V6', 'V8'].map((value) => ({ value, label: value }))} placeholder="Select an engine layout" required onChange={setLayout} /></label>
      <label><span>Aspiration Type</span><AdminSelect value={aspirationType} options={[
        { value: 'naturally_aspirated', label: 'Naturally Aspirated' },
        { value: 'supercharged', label: 'Supercharged' },
        { value: 'turbocharged', label: 'Turbocharged' },
        { value: 'twin_turbocharged', label: 'Twin Turbocharged' },
      ]} placeholder="Select an aspiration type" required onChange={setAspirationType} /></label>
      <label><span>Fuel Delivery</span><AdminSelect value={fuelDelivery} options={[
        { value: 'fuel_injected', label: 'Fuel Injected' },
        { value: 'carbureted', label: 'Carbureted' },
      ]} placeholder="Select a fuel delivery type" required onChange={setFuelDelivery} /></label>
      <label><span>Alloy Material</span><input value={alloyMaterial} maxLength={100} onChange={(event) => setAlloyMaterial(event.target.value)} required /></label>
      <label><span>Factory Code</span><input value={factoryCode} maxLength={64} onChange={(event) => setFactoryCode(event.target.value)} required /></label>
      <label><span>Displacement (Liters)</span><input type="number" min="0.001" max="20" step="any" value={displacement} onChange={(event) => setDisplacement(event.target.value)} required /></label>
      <label><span>Horsepower</span><input type="number" min="1" max="2147483647" step="1" value={horsepower} onChange={(event) => setHorsepower(event.target.value)} required /></label>
      <label><span>RPM</span><input type="number" min="1" max="2147483647" step="1" value={rpm} onChange={(event) => setRpm(event.target.value)} required /></label>
      <button className="admin-submit" disabled={busy}>{busy ? 'Creating engine…' : 'Create Engine'} <span>→</span></button>
    </form>
    {notices.length > 0 && <div className="status-stack" aria-live="polite">{notices.map((notice) =>
      <div className={`status-card ${notice.kind}`} key={notice.id}>
        <button className="status-dismiss" type="button" onClick={() => setNotices((current) => current.filter((item) => item.id !== notice.id))}>×</button>
        <strong>{notice.kind === 'success' ? 'Engine created' : 'Request failed'}</strong><p>{notice.message}</p>
      </div>,
    )}</div>}
  </div>;
}
