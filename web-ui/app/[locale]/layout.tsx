import type { Metadata } from 'next';
import { IntlayerClientProvider } from 'next-intlayer';
import { Inter } from 'next/font/google';
import { ThemeProvider } from '@/components/ThemeProvider';
import { Toaster } from '@/components/ui/toaster';
import '../globals.css';

const inter = Inter({ subsets: ['latin'] });

export const metadata: Metadata = {
  title: 'SANKEY Forex Copier',
  description: 'MT4/MT5 Trade Copier with low latency and remote control',
};

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
            {children}
            <Toaster />
          </IntlayerClientProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
