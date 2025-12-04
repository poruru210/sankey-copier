import * as React from 'react';
import { cn } from '@/lib/utils';

type SwitchLabelProps = React.LabelHTMLAttributes<HTMLLabelElement> & Record<string, unknown>;

export interface SwitchProps extends React.InputHTMLAttributes<HTMLInputElement> {
  onCheckedChange?: (checked: boolean) => void;
  ref?: React.Ref<HTMLInputElement>;
  labelProps?: SwitchLabelProps;
  isPending?: boolean;
}

function Switch({
  className,
  onCheckedChange,
  onChange,
  ref,
  labelProps,
  isPending = false,
  disabled,
  ...props
}: SwitchProps) {
  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    onChange?.(e);
    onCheckedChange?.(e.target.checked);
  };

  return (
    <label
      {...labelProps}
      className={cn(
        'inline-flex items-center cursor-pointer select-none relative',
        isPending && 'opacity-60 cursor-wait pointer-events-none',
        labelProps?.className
      )}
      aria-busy={isPending || undefined}
      data-pending={isPending ? 'true' : undefined}
    >
      <input
        type="checkbox"
        className="sr-only peer"
        ref={ref}
        onChange={handleChange}
        disabled={isPending || disabled}
        {...props}
      />
      <div
        className={cn(
          'relative w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[\'\'] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600',
          className
        )}
      />
      {isPending && (
        <span className="absolute inset-0 flex items-center justify-center pointer-events-none" aria-hidden>
          <span className="h-3 w-3 rounded-full border-2 border-blue-500 border-t-transparent dark:border-blue-200 animate-spin" />
        </span>
      )}
    </label>
  );
}

export { Switch };
