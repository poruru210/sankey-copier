import * as React from 'react';
import { cn } from '@/lib/utils';
import { X } from 'lucide-react';

interface SheetProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: React.ReactNode;
  side?: 'left' | 'right' | 'bottom';
  className?: string;
}

const Sheet: React.FC<SheetProps> = ({ open, onOpenChange, children, side = 'right', className }) => {
  // Close on Escape key
  React.useEffect(() => {
    if (!open) return;

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onOpenChange(false);
      }
    };

    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, [open, onOpenChange]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[100]">
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/50 transition-opacity duration-300 animate-in fade-in pointer-events-auto"
        onClick={() => onOpenChange(false)}
        aria-hidden="true"
      />

      {/* Sheet Content */}
      <div
        className={cn(
          'fixed bg-background shadow-lg transition-transform duration-300 pointer-events-auto z-[100]',
          // Position based on side
          side === 'left' && 'left-0 top-0 bottom-0 w-[80%] max-w-sm',
          side === 'right' && 'right-0 top-0 bottom-0 w-[80%] max-w-sm',
          side === 'bottom' && 'left-0 right-0 bottom-0 h-[92vh] rounded-t-lg',
          // Transform based on open state and side
          open
            ? side === 'bottom'
              ? 'translate-y-0'
              : 'translate-x-0'
            : side === 'left'
            ? '-translate-x-full'
            : side === 'right'
            ? 'translate-x-full'
            : 'translate-y-full',
          className
        )}
        onClick={(e) => e.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
};

const SheetContent = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, children, ...props }, ref) => (
    <div
      ref={ref}
      className={cn('relative h-full flex flex-col', className)}
      {...props}
    >
      {children}
    </div>
  )
);
SheetContent.displayName = 'SheetContent';

const SheetHeader = ({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    className={cn('flex items-center justify-between px-4 py-3 border-b', className)}
    {...props}
  />
);
SheetHeader.displayName = 'SheetHeader';

const SheetTitle = React.forwardRef<HTMLHeadingElement, React.HTMLAttributes<HTMLHeadingElement>>(
  ({ className, ...props }, ref) => (
    <h2 ref={ref} className={cn('text-lg font-semibold', className)} {...props} />
  )
);
SheetTitle.displayName = 'SheetTitle';

interface SheetCloseProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  onClose: () => void;
}

const SheetClose: React.FC<SheetCloseProps> = ({ onClose, className, ...props }) => (
  <button
    onClick={onClose}
    className={cn(
      'rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100',
      'focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
      'disabled:pointer-events-none',
      className
    )}
    {...props}
  >
    <X className="h-4 w-4" />
    <span className="sr-only">Close</span>
  </button>
);
SheetClose.displayName = 'SheetClose';

export { Sheet, SheetContent, SheetHeader, SheetTitle, SheetClose };
