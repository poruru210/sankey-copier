export { intlayerProxy as proxy } from 'next-intlayer/proxy';

export const config = {
  matcher: [
    '/',
    '/(en|ja)',
    '/(en|ja)/:path((?!installations).*)*',
  ],
};
