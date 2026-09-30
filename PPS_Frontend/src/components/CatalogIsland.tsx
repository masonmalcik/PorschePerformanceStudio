import { useEffect, useMemo, useState } from 'react';
import { catalogApiBase } from '../lib/api';
import { addCartItem } from '../lib/cart';

type ApiProduct = {
  id: string;
  sku: string;
  modelNumber: string | null;
  price: { amount: string; currency?: string };
  name: string;
  description: string | null;
  imageName: string;
  imageUrls: string[];
  saleType: string;
};

type ProductPage = { items: ApiProduct[] };
type ApiCategory = { id: string; name: string; description: string | null; parentId: string | null };
const titleCase = (value: string) => value.replaceAll('_', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase());

export default function CatalogIsland() {
  const [products, setProducts] = useState<ApiProduct[]>([]);
  const [categories, setCategories] = useState<ApiCategory[]>([]);
  const [saleType, setSaleType] = useState('All');
  const [query, setQuery] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');

  useEffect(() => {
    const controller = new AbortController();
    const params = new URLSearchParams({ pageSize: '100', sort: 'name', direction: 'asc' });
    Promise.all([
      fetch(`${catalogApiBase}/products?${params}`, { signal: controller.signal }),
      fetch(`${catalogApiBase}/categories`, { signal: controller.signal }),
    ])
      .then(async ([productResponse, categoryResponse]) => {
        if (!productResponse.ok) throw new Error(`Catalog request failed with status ${productResponse.status}.`);
        if (!categoryResponse.ok) throw new Error(`Category request failed with status ${categoryResponse.status}.`);
        return Promise.all([productResponse.json() as Promise<ProductPage>, categoryResponse.json() as Promise<ApiCategory[]>]);
      })
      .then(([page, categoryValues]) => { setProducts(page.items); setCategories(categoryValues); })
      .catch((reason: unknown) => {
        if (reason instanceof DOMException && reason.name === 'AbortError') return;
        setError(reason instanceof Error ? reason.message : 'The catalog is temporarily unavailable.');
      })
      .finally(() => setLoading(false));
    return () => controller.abort();
  }, []);

  const saleTypes = useMemo(() => ['All', ...new Set(products.map((product) => titleCase(product.saleType)))], [products]);
  const visible = useMemo(() => products.filter((product) =>
    (saleType === 'All' || titleCase(product.saleType) === saleType) &&
    `${product.name} ${product.modelNumber ?? ''} ${product.description ?? ''}`.toLowerCase().includes(query.toLowerCase())
  ), [products, saleType, query]);
  const rootCategories = useMemo(() => categories.filter((category) => !category.parentId), [categories]);

  return <section className="catalog-shell" aria-label="Performance product catalog">
    {rootCategories.length > 0 && <nav className="catalog-category-grid" aria-label="Product categories">
      {rootCategories.map((category) => <a href="#" onClick={(event) => event.preventDefault()} key={category.id}>
        <h2>{category.name}</h2>
        <p>{category.description ?? 'Explore performance upgrades in this category.'}</p>
        <span aria-hidden="true">→</span>
      </a>)}
    </nav>}
    <div className="catalog-tools">
      <div className="filters" aria-label="Filter products by sale type">
        {saleTypes.map((item) => <button key={item} className={saleType === item ? 'selected' : ''} onClick={() => setSaleType(item)}>{item}</button>)}
      </div>
      <label className="search"><span className="sr-only">Search catalog</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search product or model number" /><span aria-hidden="true">⌕</span></label>
    </div>
    {loading ? <div className="empty-state" role="status"><h2>Loading catalog…</h2></div> : error ?
      <div className="empty-state" role="alert"><h2>Catalog unavailable</h2><p>{error}</p></div> : <>
        <p className="results-count">{visible.length} {visible.length === 1 ? 'upgrade' : 'upgrades'}</p>
        {visible.length ? <div className="product-grid">
          {visible.map((product) => <article className="product-card" key={product.id}>
            <div className="product-image"><img src={product.imageUrls[0] ?? '/favicon.svg'} alt="" loading="lazy" /><span>{product.modelNumber ?? product.sku}</span></div>
            <div className="product-copy"><p>{titleCase(product.saleType)}</p><h2>{product.name}</h2><div><span>{product.description ?? product.imageName}</span><strong>{new Intl.NumberFormat('en-US', { style: 'currency', currency: product.price.currency ?? 'USD' }).format(Number(product.price.amount))}</strong></div><button className="card-action" onClick={() => void addCartItem(product.id, product.price.amount).then(() => setNotice(`${product.name} added to cart.`)).catch((reason: unknown) => setNotice(reason instanceof Error ? reason.message : 'Unable to add product.'))}>Add to cart</button></div>
          </article>)}
        </div> : <div className="empty-state"><h2>No matching upgrades</h2><p>Try a different sale type or search term.</p></div>}
      </>}{notice&&<div className="catalog-notice" role="status">{notice}<button onClick={()=>setNotice('')}>×</button></div>}
  </section>;
}
