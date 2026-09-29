import { useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { catalogAdminApiBase, developmentAdminKey } from '../lib/api';

const endpoint = `${catalogAdminApiBase}/vehicle-models`;
const adminKey = developmentAdminKey;

type CreatedVehicleModel = {
  name: string;
};

type StatusNotice = {
  key: number;
  kind: 'error' | 'success';
  title: string;
  message: string;
};

export default function CreateVehicleModelForm() {
  const [name, setName] = useState('');
  const [modelCode, setModelCode] = useState('');
  const [validationErrors, setValidationErrors] = useState({ name: false, modelCode: false });
  const [notices, setNotices] = useState<StatusNotice[]>([]);
  const nextNoticeKey = useRef(1);
  const [isSubmitting, setIsSubmitting] = useState(false);

  function addNotice(kind: StatusNotice['kind'], title: string, message: string) {
    const notice = { key: nextNoticeKey.current++, kind, title, message };
    setNotices((current) => [notice, ...current]);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmedName = name.trim();
    const trimmedModelCode = modelCode.trim();
    if (!trimmedName || !trimmedModelCode) {
      setValidationErrors({ name: !trimmedName, modelCode: !trimmedModelCode });
      return;
    }

    setIsSubmitting(true);
    try {
      const response = await fetch(endpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-pps-admin-key': adminKey,
        },
        body: JSON.stringify({ name: trimmedName, modelCode: trimmedModelCode }),
      });

      const rawBody = await response.text();
      let responseBody: unknown = null;
      if (rawBody) {
        try { responseBody = JSON.parse(rawBody); } catch { responseBody = rawBody; }
      }

      if (!response.ok) {
        if (response.status === 409) {
          throw new Error(`${trimmedName} already exists.`);
        }
        const nestedError = typeof responseBody === 'object' && responseBody && 'error' in responseBody
          ? responseBody.error
          : null;
        const apiMessage = typeof nestedError === 'object' && nestedError && 'message' in nestedError
          ? String(nestedError.message)
          : typeof responseBody === 'object' && responseBody && 'message' in responseBody
            ? String(responseBody.message)
          : typeof responseBody === 'string' && responseBody
            ? responseBody
            : `The API returned ${response.status} ${response.statusText}.`;
        throw new Error(apiMessage);
      }

      const createdModel = responseBody as CreatedVehicleModel;
      addNotice('success', 'Model created', `${createdModel.name} was created successfully.`);
      setName('');
      setModelCode('');
      setValidationErrors({ name: false, modelCode: false });
    } catch (requestError) {
      addNotice(
        'error',
        'Request failed',
        requestError instanceof Error ? requestError.message : 'The request could not be completed.',
      );
    } finally {
      setIsSubmitting(false);
    }
  }

  const nameInvalid = validationErrors.name;
  const modelCodeInvalid = validationErrors.modelCode;

  return (
    <div className="admin-workspace">
      <form className="admin-form" onSubmit={handleSubmit}>
        <div className="form-heading">
          <h2>Model Details</h2>
        </div>

        <label className={`form-field${nameInvalid ? ' invalid' : ''}`}>
          <span>Model Name</span>
          <input autoComplete="off" value={name} onChange={(event) => { setName(event.target.value); setValidationErrors((current) => ({ ...current, name: false })); }} aria-invalid={nameInvalid} aria-describedby={nameInvalid ? 'model-name-error' : undefined} />
          {nameInvalid && <small className="field-error" id="model-name-error">This Field is Required</small>}
        </label>
        <label className={`form-field${modelCodeInvalid ? ' invalid' : ''}`}>
          <span>Model Code</span>
          <input autoComplete="off" value={modelCode} onChange={(event) => { setModelCode(event.target.value); setValidationErrors((current) => ({ ...current, modelCode: false })); }} aria-invalid={modelCodeInvalid} aria-describedby={modelCodeInvalid ? 'model-code-error' : undefined} />
          {modelCodeInvalid && <small className="field-error" id="model-code-error">This Field is Required</small>}
        </label>

        <button className="admin-submit" type="submit" disabled={isSubmitting}>
          {isSubmitting ? 'Creating model…' : 'Create Model'} <span aria-hidden="true">→</span>
        </button>
      </form>

      {notices.length > 0 && (
        <div className={`status-stack${notices.length > 10 ? ' scrollable' : ''}`} aria-label="Submission results" aria-live="polite">
          {notices.map((notice) => (
            <div className={`status-card ${notice.kind}`} role={notice.kind === 'error' ? 'alert' : 'status'} key={notice.key}>
              <button
                className="status-dismiss"
                type="button"
                onClick={() => setNotices((current) => current.filter((item) => item.key !== notice.key))}
                aria-label={`Dismiss ${notice.title.toLowerCase()} notification`}
              >×</button>
              <strong>{notice.title}</strong>
              <p>{notice.message}</p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
