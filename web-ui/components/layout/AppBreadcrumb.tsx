'use client';

// AppBreadcrumb component - displays navigation breadcrumbs based on current path
// Uses usePathname to determine current location and generates appropriate breadcrumb trail

import { usePathname } from 'next/navigation';
import Link from 'next/link';
import { useIntlayer } from 'next-intlayer';
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from '@/components/ui/breadcrumb';

// Route configuration for breadcrumb labels
interface BreadcrumbConfig {
  labelKey: string;
  href?: string;
}

export function AppBreadcrumb() {
  const pathname = usePathname();
  const content = useIntlayer('breadcrumb');

  // Extract locale and path segments
  const segments = pathname.split('/').filter(Boolean);
  const locale = segments[0] || 'en';
  const pathSegments = segments.slice(1); // Remove locale

  // Build breadcrumb items based on path
  const breadcrumbs = buildBreadcrumbs(pathSegments, locale, content);

  if (breadcrumbs.length === 0) {
    return (
      <Breadcrumb>
        <BreadcrumbList>
          <BreadcrumbItem>
            <BreadcrumbPage>{content.home}</BreadcrumbPage>
          </BreadcrumbItem>
        </BreadcrumbList>
      </Breadcrumb>
    );
  }

  return (
    <Breadcrumb>
      <BreadcrumbList>
        {/* Home link */}
        <BreadcrumbItem>
          <BreadcrumbLink asChild>
            <Link href={`/${locale}/connections`}>{content.home}</Link>
          </BreadcrumbLink>
        </BreadcrumbItem>

        {/* Path segments */}
        {breadcrumbs.map((crumb, index) => {
          const isLast = index === breadcrumbs.length - 1;

          return (
            <span key={crumb.href || index} className="contents">
              <BreadcrumbSeparator />
              <BreadcrumbItem>
                {isLast ? (
                  <BreadcrumbPage>{crumb.label}</BreadcrumbPage>
                ) : (
                  <BreadcrumbLink asChild>
                    <Link href={crumb.href!}>{crumb.label}</Link>
                  </BreadcrumbLink>
                )}
              </BreadcrumbItem>
            </span>
          );
        })}
      </BreadcrumbList>
    </Breadcrumb>
  );
}

interface BreadcrumbItem {
  label: string;
  href?: string;
}

// Build breadcrumb items from path segments
function buildBreadcrumbs(
  segments: string[],
  locale: string,
  content: Record<string, string>
): BreadcrumbItem[] {
  if (segments.length === 0) {
    return [];
  }

  const breadcrumbs: BreadcrumbItem[] = [];
  let currentPath = `/${locale}`;

  for (let i = 0; i < segments.length; i++) {
    const segment = segments[i];
    currentPath += `/${segment}`;

    // Get label for this segment
    const label = getSegmentLabel(segment, segments, i, content);

    breadcrumbs.push({
      label,
      href: currentPath,
    });
  }

  return breadcrumbs;
}

// Get display label for a path segment
function getSegmentLabel(
  segment: string,
  allSegments: string[],
  index: number,
  content: Record<string, string>
): string {
  // Known route mappings
  const routeLabels: Record<string, string> = {
    connections: content.connections,
    installations: content.installations,
    sites: content.sites,
    'trade-groups': content.tradeGroups,
    settings: content.settings,
  };

  // Check if this is a known route
  if (routeLabels[segment]) {
    return routeLabels[segment];
  }

  // If previous segment was 'trade-groups', this is a trade group ID
  if (index > 0 && allSegments[index - 1] === 'trade-groups') {
    // Decode the ID (it may be URL encoded)
    return decodeURIComponent(segment);
  }

  // Default: use segment as-is (capitalized)
  return segment.charAt(0).toUpperCase() + segment.slice(1);
}
