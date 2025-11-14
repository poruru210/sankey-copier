/**
 * Root locale page - redirects to connections page
 *
 * When users access /{locale} directly, redirect them to /{locale}/connections
 * as the connections page is the main entry point of the application.
 */
import { redirect } from 'next/navigation';

export default async function LocaleRootPage({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;

  // Redirect to connections page as the default landing page
  redirect(`/${locale}/connections`);
}
