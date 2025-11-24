/**
 * Stepper component for multi-step forms
 *
 * Provides a visual indicator and navigation for multi-step workflows.
 * Built with shadcn/ui styling conventions.
 */

import * as React from "react";
import { Check } from "lucide-react";
import { cn } from "@/lib/utils";

// ============================================================================
// Types
// ============================================================================

export interface Step {
  id: string;
  label: string;
  description?: string;
  optional?: boolean;
}

interface StepperContextValue {
  steps: Step[];
  currentStep: number;
  setCurrentStep: (step: number) => void;
  isStepComplete: (step: number) => boolean;
  isStepClickable: (step: number) => boolean;
  completedSteps: Set<number>;
  setStepComplete: (step: number, complete: boolean) => void;
}

const StepperContext = React.createContext<StepperContextValue | undefined>(
  undefined
);

// ============================================================================
// Hooks
// ============================================================================

function useStepper() {
  const context = React.useContext(StepperContext);
  if (!context) {
    throw new Error("useStepper must be used within StepperProvider");
  }
  return context;
}

// ============================================================================
// Stepper Provider
// ============================================================================

interface StepperProps {
  children: React.ReactNode;
  initialStep?: number;
  steps: Step[];
  allowStepNavigation?: boolean;
  onStepChange?: (step: number) => void;
}

export function Stepper({
  children,
  initialStep = 0,
  steps,
  allowStepNavigation = false,
  onStepChange,
}: StepperProps) {
  const [currentStep, setCurrentStepState] = React.useState(initialStep);
  const [completedSteps, setCompletedStepsState] = React.useState<Set<number>>(
    new Set()
  );

  const setCurrentStep = React.useCallback(
    (step: number) => {
      setCurrentStepState(step);
      onStepChange?.(step);
    },
    [onStepChange]
  );

  const isStepComplete = React.useCallback(
    (step: number) => completedSteps.has(step),
    [completedSteps]
  );

  const isStepClickable = React.useCallback(
    (step: number) => {
      if (!allowStepNavigation) return false;
      // Allow clicking on completed steps or the next step after completed ones
      if (step < currentStep) return true;
      if (step === currentStep) return true;
      // Check if all previous steps are completed
      for (let i = 0; i < step; i++) {
        if (!completedSteps.has(i)) return false;
      }
      return true;
    },
    [allowStepNavigation, currentStep, completedSteps]
  );

  const setStepComplete = React.useCallback(
    (step: number, complete: boolean) => {
      setCompletedStepsState((prev) => {
        const next = new Set(prev);
        if (complete) {
          next.add(step);
        } else {
          next.delete(step);
        }
        return next;
      });
    },
    []
  );

  const value: StepperContextValue = React.useMemo(
    () => ({
      steps,
      currentStep,
      setCurrentStep,
      isStepComplete,
      isStepClickable,
      completedSteps,
      setStepComplete,
    }),
    [
      steps,
      currentStep,
      setCurrentStep,
      isStepComplete,
      isStepClickable,
      completedSteps,
      setStepComplete,
    ]
  );

  return (
    <StepperContext.Provider value={value}>{children}</StepperContext.Provider>
  );
}

// ============================================================================
// Stepper Header (Step Indicators)
// ============================================================================

export function StepperHeader({ className }: { className?: string }) {
  const { steps, currentStep, isStepComplete, isStepClickable, setCurrentStep } =
    useStepper();

  return (
    <div className={cn("w-full", className)}>
      <div className="flex items-center justify-between">
        {steps.map((step, index) => {
          const isActive = index === currentStep;
          const isComplete = isStepComplete(index);
          const isClickable = isStepClickable(index);

          return (
            <React.Fragment key={step.id}>
              {/* Step Indicator */}
              <div className="flex flex-col items-center flex-1">
                <button
                  type="button"
                  onClick={() => isClickable && setCurrentStep(index)}
                  disabled={!isClickable}
                  className={cn(
                    "flex h-10 w-10 items-center justify-center rounded-full border-2 transition-all",
                    {
                      "border-primary bg-primary text-primary-foreground":
                        isActive || isComplete,
                      "border-muted-foreground/25 bg-background text-muted-foreground":
                        !isActive && !isComplete,
                      "cursor-pointer hover:border-primary hover:bg-primary/10":
                        isClickable && !isActive,
                      "cursor-not-allowed": !isClickable,
                    }
                  )}
                >
                  {isComplete ? (
                    <Check className="h-5 w-5" />
                  ) : (
                    <span className="text-sm font-medium">{index + 1}</span>
                  )}
                </button>
                <div className="mt-2 text-center">
                  <p
                    className={cn("text-sm font-medium", {
                      "text-primary": isActive,
                      "text-foreground": isComplete && !isActive,
                      "text-muted-foreground": !isActive && !isComplete,
                    })}
                  >
                    {step.label}
                  </p>
                  {step.description && (
                    <p className="text-xs text-muted-foreground mt-0.5">
                      {step.description}
                    </p>
                  )}
                </div>
              </div>

              {/* Connector Line */}
              {index < steps.length - 1 && (
                <div className="flex-1 mx-2 -mt-12">
                  <div
                    className={cn(
                      "h-0.5 transition-colors",
                      isComplete
                        ? "bg-primary"
                        : "bg-muted-foreground/25"
                    )}
                  />
                </div>
              )}
            </React.Fragment>
          );
        })}
      </div>
    </div>
  );
}

// ============================================================================
// Stepper Content
// ============================================================================

interface StepperContentProps {
  children: React.ReactNode;
  className?: string;
}

export function StepperContent({ children, className }: StepperContentProps) {
  return <div className={cn("mt-8", className)}>{children}</div>;
}

// ============================================================================
// Step Component
// ============================================================================

interface StepProps {
  children: React.ReactNode;
  stepIndex: number;
}

export function Step({ children, stepIndex }: StepProps) {
  const { currentStep } = useStepper();

  if (currentStep !== stepIndex) return null;

  return <>{children}</>;
}

// ============================================================================
// Stepper Actions (Navigation Buttons)
// ============================================================================

interface StepperActionsProps {
  children: React.ReactNode;
  className?: string;
}

export function StepperActions({ children, className }: StepperActionsProps) {
  return (
    <div className={cn("flex items-center justify-between mt-6", className)}>
      {children}
    </div>
  );
}

// Export hook for use in components
export { useStepper };
