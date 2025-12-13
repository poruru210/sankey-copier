// Root layout for locale routes
// ThemeProvider wraps the entire app
// LayoutWrapper internally provides ServerLogProvider and shadcn SidebarProvider

import type { Metadata } from 'next';
import { IntlayerClientProvider } from 'next-intlayer';
import { Inter } from 'next/font/google';
import { ThemeProvider } from '@/components/layout/ThemeProvider';
import { LayoutWrapper } from '@/components/layout/LayoutWrapper';
import { Toaster } from '@/components/ui/toaster';
import '../globals.css';

const inter = Inter({ subsets: ['latin'] });

export const metadata: Metadata = {
  title: 'SANKEY Copier',
  description: 'MT4/MT5 Trade Copier with low latency and remote control',
  icons: {
    icon: [
      { url: '/favicon-16x16.png', sizes: '16x16', type: 'image/png' },
      { url: '/favicon-32x32.png', sizes: '32x32', type: 'image/png' },
      { url: '/favicon-48x48.png', sizes: '48x48', type: 'image/png' },
      { url: '/favicon.ico', sizes: 'any' },
    ],
    apple: [
      { url: '/apple-touch-icon.png', sizes: '180x180', type: 'image/png' },
    ],
    other: [
      { rel: 'android-chrome-192x192', url: '/android-chrome-192x192.png' },
      { rel: 'android-chrome-512x512', url: '/android-chrome-512x512.png' },
    ],
  },
  manifest: '/site.webmanifest',
};

// Generate static pages for each locale at build time
// Required for Next.js static export with dynamic [locale] route
export async function generateStaticParams() {
  return [
    { locale: 'en' },
    { locale: 'ja' },
  ];
}

export default async function RootLayout({
  children,
  params,
}: {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;

  return (
    <html lang={locale} suppressHydrationWarning>
      <body className={inter.className}>
        <ThemeProvider
          attribute="class"
          defaultTheme="system"
          enableSystem
          disableTransitionOnChange
        >
          <IntlayerClientProvider locale={locale}>
            <LayoutWrapper>
              {children}
            </LayoutWrapper>
            <Toaster />
          </IntlayerClientProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
