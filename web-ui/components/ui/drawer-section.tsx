// DrawerSection - Unified layout components for drawer content
// Provides consistent styling across all drawer/modal forms
// Structure: DrawerSection > DrawerSectionHeader > DrawerSectionContent

import * as React from 'react';
import { cn } from '@/lib/utils';
import { Typography, Caption } from '@/components/ui/typography';
import { Label } from '@/components/ui/label';

/**
 * DrawerSection - Container for a logical section within a drawer
 * Adds consistent spacing and optional border separator
 */
interface DrawerSectionProps {
  children: React.ReactNode;
  className?: string;
  /** Whether to show a top border separator */
  bordered?: boolean;
}

export function DrawerSection({
  children,
  className,
  bordered = false,
}: DrawerSectionProps) {
  return (
    <div
      className={cn(
        'space-y-3',
        bordered && 'pt-4 border-t border-border',
        className
      )}
    >
      {children}
    </div>
  );
}

/**
 * DrawerSectionHeader - Title and description for a section
 */
interface DrawerSectionHeaderProps {
  /** Section title */
  title: string;
  /** Optional description text */
  description?: string;
  className?: string;
}

export function DrawerSectionHeader({
  title,
  description,
  className,
}: DrawerSectionHeaderProps) {
  return (
    <div className={cn('space-y-1', className)}>
      <Typography variant="large" as="h3">
        {title}
      </Typography>
      {description && <Caption>{description}</Caption>}
    </div>
  );
}

/**
 * DrawerSectionContent - Content area within a section
 */
interface DrawerSectionContentProps {
  children: React.ReactNode;
  className?: string;
}

export function DrawerSectionContent({
  children,
  className,
}: DrawerSectionContentProps) {
  return (
    <div className={cn('space-y-4', className)}>
      {children}
    </div>
  );
}

/**
 * DrawerFormField - Consistent form field with label and optional description
 */
interface DrawerFormFieldProps {
  label: string;
  description?: string;
  children: React.ReactNode;
  className?: string;
  htmlFor?: string;
}

export function DrawerFormField({
  label,
  description,
  children,
  className,
  htmlFor,
}: DrawerFormFieldProps) {
  return (
    <div className={cn('space-y-1.5', className)}>
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
      {description && <Caption>{description}</Caption>}
    </div>
  );
}

/**
 * DrawerInfoCard - Display-only card for showing connection or account info
 */
interface DrawerInfoCardProps {
  children: React.ReactNode;
  className?: string;
}

export function DrawerInfoCard({
  children,
  className,
}: DrawerInfoCardProps) {
  return (
    <div className={cn('p-3 bg-muted rounded-lg border', className)}>
      {children}
    </div>
  );
}
