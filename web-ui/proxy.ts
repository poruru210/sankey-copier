export { intlayerProxy as proxy } from 'next-intlayer/proxy';

export const config = {
  // Match all routes except static files, APIs, and Next.js internals
  matcher:
    '/((?!api|static|assets|robots|sitemap|sw|service-worker|manifest|.*\\..*|_next).*)',
};
