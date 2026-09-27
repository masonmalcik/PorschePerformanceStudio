import { defineConfig } from 'astro/config';
import react from '@astrojs/react';

const githubPages = process.env.GITHUB_PAGES === 'true';

export default defineConfig({
  site: 'https://masonmalcik.github.io',
  base: githubPages ? '/PorschePerformanceStudio' : '/',
  integrations: [react()],
});
