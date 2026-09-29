import { useEffect, useRef, useState } from 'react';
import type { FormEvent, KeyboardEvent } from 'react';
import AdminSelect from './AdminSelect';
import { catalogAdminApiBase, developmentAdminKey } from '../lib/api';

const api = catalogAdminApiBase;
const key = developmentAdminKey;
type Notice = { key: number; kind: 'error' | 'success'; title: string; message: string };
type Option = { id: string; name: string };

export default function CreateProductForm() {
  const [sku, setSku] = useState(''), [modelNumber, setModelNumber] = useState(''), [brandId, setBrandId] = useState(''), [name, setName] = useState(''), [amount, setAmount] = useState(''), [imageName, setImageName] = useState(''), [description, setDescription] = useState(''), [saleType, setSaleType] = useState(''), [categoryId, setCategoryId] = useState('');
  const [attributes, setAttributes] = useState<string[]>([]), [attributeDraft, setAttributeDraft] = useState('');
  const [brands, setBrands] = useState<Option[]>([]), [categories, setCategories] = useState<Option[]>([]), [notices, setNotices] = useState<Notice[]>([]), [submitting, setSubmitting] = useState(false);
  const next = useRef(1);
  const notify = (kind: Notice['kind'], title: string, message: string) => setNotices(current => [{ key: next.current++, kind, title, message }, ...current]);

  useEffect(() => { Promise.all([fetch(`${api}/brands`, { headers: { 'x-pps-admin-key': key } }), fetch(`${api}/categories`, { headers: { 'x-pps-admin-key': key } })]).then(async ([brandResponse, categoryResponse]) => { if (!brandResponse.ok || !categoryResponse.ok) throw new Error('Unable to load catalog selections.'); setBrands(await brandResponse.json()); setCategories(await categoryResponse.json()); }).catch(error => notify('error', 'Catalog selections unavailable', error.message)); }, []);

  function addAttribute() { const attribute = attributeDraft.trim(); if (!attribute || attributes.includes(attribute)) return; setAttributes(current => [...current, attribute]); setAttributeDraft(''); }
  function attributeKeyDown(event: KeyboardEvent<HTMLInputElement>) { if (event.key === 'Enter') { event.preventDefault(); addAttribute(); } }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!saleType) { notify('error', 'Sale type required', 'Select a sale type before creating the product.'); return; }
    if (!categoryId) { notify('error', 'Category required', 'Select a category before creating the product.'); return; }
    setSubmitting(true);
    try {
      const response = await fetch(`${api}/products`, { method: 'POST', headers: { 'Content-Type': 'application/json', 'x-pps-admin-key': key }, body: JSON.stringify({ sku: sku.trim(), modelNumber: modelNumber.trim(), brandId, price: { amount }, name: name.trim(), attributes, description: description.trim(), imageName: imageName.trim(), imageUrls: [], saleType, categoryIds: [categoryId] }) });
      const body = await response.json();
      if (!response.ok) throw new Error(response.status === 409 ? `${sku.trim()} already exists.` : body?.error?.message ?? 'The product could not be created.');
      notify('success', 'Product created', `${body.name} was created successfully.`);
      setSku(''); setModelNumber(''); setBrandId(''); setName(''); setAmount(''); setImageName(''); setDescription(''); setCategoryId(''); setSaleType(''); setAttributes([]); setAttributeDraft('');
    } catch (error) { notify('error', 'Request failed', error instanceof Error ? error.message : 'The request could not be completed.'); } finally { setSubmitting(false); }
  }

  return <div className="admin-workspace"><form className="admin-form" onSubmit={submit}><div className="form-heading"><h2>Product Details</h2></div>
    <label><span>Brand</span><AdminSelect value={brandId} options={brands.map(item => ({ value: item.id, label: item.name }))} placeholder="Select a brand" onChange={setBrandId} /></label>
    <label><span>Category</span><AdminSelect value={categoryId} options={categories.map(item => ({ value: item.id, label: item.name }))} placeholder="Select a category" onChange={setCategoryId} /></label>
    <label><span>Sale Type</span><AdminSelect value={saleType} options={[{ value: 'retail', label: 'Retail' }, { value: 'sale', label: 'Sale' }, { value: 'clearance', label: 'Clearance' }]} placeholder="Select a sale type" onChange={setSaleType} /></label>
    <label><span>Product Name</span><input value={name} onChange={event => setName(event.target.value)} required /></label><label><span>SKU</span><input value={sku} onChange={event => setSku(event.target.value)} required /></label><label><span>Model Number</span><input value={modelNumber} onChange={event => setModelNumber(event.target.value)} required /></label><label><span>Price (USD)</span><input type="number" min="0" step="0.01" value={amount} onChange={event => setAmount(event.target.value)} required /></label>
    <label><span>Attributes</span><div className="admin-tag-entry"><input value={attributeDraft} maxLength={100} placeholder="Enter an attribute" onChange={event => setAttributeDraft(event.target.value)} onKeyDown={attributeKeyDown} /><button type="button" onClick={addAttribute}>Add</button></div></label>
    {attributes.length > 0 && <div className="admin-selection-tags" aria-label="Product attributes">{attributes.map(attribute => <span className="admin-selection-tag" key={attribute}>{attribute}<button type="button" aria-label={`Remove ${attribute}`} onClick={() => setAttributes(current => current.filter(value => value !== attribute))}>×</button></span>)}</div>}
    <label><span>Image Filename</span><input value={imageName} onChange={event => setImageName(event.target.value)} required /></label><label><span>Description</span><textarea value={description} onChange={event => setDescription(event.target.value)} required /></label><button className="admin-submit" disabled={submitting}>{submitting ? 'Creating product…' : 'Create Product'}</button>
  </form>{notices.length > 0 && <div className="status-stack">{notices.map(item => <div className={`status-card ${item.kind}`} key={item.key}><button className="status-dismiss" type="button" onClick={() => setNotices(current => current.filter(value => value.key !== item.key))}>×</button><strong>{item.title}</strong><p>{item.message}</p></div>)}</div>}</div>;
}
