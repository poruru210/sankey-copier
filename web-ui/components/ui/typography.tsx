// Typography component for consistent text styling across the application.
// Based on shadcn/ui typography guidelines with project-specific adjustments.
// Provides semantic heading levels (h1-h4) and text variants (p, lead, muted, etc.)

import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '@/lib/utils';

// Valid HTML element types for typography
type TypographyElement = 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6' | 'p' | 'span' | 'small' | 'blockquote' | 'code';

// Typography variants using CVA for consistent styling
const typographyVariants = cva('', {
  variants: {
    variant: {
      h1: 'scroll-m-20 text-4xl font-extrabold tracking-tight lg:text-5xl',
      h2: 'scroll-m-20 border-b pb-2 text-3xl font-semibold tracking-tight first:mt-0',
      h3: 'scroll-m-20 text-2xl font-semibold tracking-tight',
      h4: 'scroll-m-20 text-xl font-semibold tracking-tight',
      p: 'leading-7 [&:not(:first-child)]:mt-6',
      lead: 'text-xl text-muted-foreground',
      large: 'text-lg font-semibold',
      small: 'text-sm font-medium leading-none',
      muted: 'text-sm text-muted-foreground',
      blockquote: 'mt-6 border-l-2 pl-6 italic',
      code: 'relative rounded bg-muted px-[0.3rem] py-[0.2rem] font-mono text-sm font-semibold',
    },
  },
  defaultVariants: {
    variant: 'p',
  },
});

// Map variants to their semantic HTML elements
const variantElementMap: Record<string, TypographyElement> = {
  h1: 'h1',
  h2: 'h2',
  h3: 'h3',
  h4: 'h4',
  p: 'p',
  lead: 'p',
  large: 'p',
  small: 'small',
  muted: 'p',
  blockquote: 'blockquote',
  code: 'code',
};

export interface TypographyProps
  extends React.HTMLAttributes<HTMLElement>,
    VariantProps<typeof typographyVariants> {
  // Optional override for the rendered HTML element
  as?: TypographyElement;
}

const Typography = React.forwardRef<HTMLElement, TypographyProps>(
  ({ className, variant = 'p', as, children, ...props }, ref) => {
    const Component = as || variantElementMap[variant || 'p'] || 'p';

    return React.createElement(
      Component,
      {
        ref,
        className: cn(typographyVariants({ variant }), className),
        ...props,
      },
      children
    );
  }
);
Typography.displayName = 'Typography';

// Convenience components for common use cases
const H1 = React.forwardRef<HTMLHeadingElement, Omit<TypographyProps, 'variant'>>(
  ({ className, ...props }, ref) => (
    <Typography ref={ref} variant="h1" className={className} {...props} />
  )
);
H1.displayName = 'H1';

const H2 = React.forwardRef<HTMLHeadingElement, Omit<TypographyProps, 'variant'>>(
  ({ className, ...props }, ref) => (
    <Typography ref={ref} variant="h2" className={className} {...props} />
  )
);
H2.displayName = 'H2';

const H3 = React.forwardRef<HTMLHeadingElement, Omit<TypographyProps, 'variant'>>(
  ({ className, ...props }, ref) => (
    <Typography ref={ref} variant="h3" className={className} {...props} />
  )
);
H3.displayName = 'H3';

const H4 = React.forwardRef<HTMLHeadingElement, Omit<TypographyProps, 'variant'>>(
  ({ className, ...props }, ref) => (
    <Typography ref={ref} variant="h4" className={className} {...props} />
  )
);
H4.displayName = 'H4';

const Lead = React.forwardRef<HTMLParagraphElement, Omit<TypographyProps, 'variant'>>(
  ({ className, ...props }, ref) => (
    <Typography ref={ref} variant="lead" className={className} {...props} />
  )
);
Lead.displayName = 'Lead';

const Muted = React.forwardRef<HTMLParagraphElement, Omit<TypographyProps, 'variant'>>(
  ({ className, ...props }, ref) => (
    <Typography ref={ref} variant="muted" className={className} {...props} />
  )
);
Muted.displayName = 'Muted';

export { Typography, typographyVariants, H1, H2, H3, H4, Lead, Muted };
