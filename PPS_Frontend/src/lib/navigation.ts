const base = import.meta.env.BASE_URL.replace(/\/$/, '');

export function appPath(path = '/'): string {
  const normalized = path.startsWith('/') ? path : `/${path}`;
  return `${base}${normalized}` || '/';
}
