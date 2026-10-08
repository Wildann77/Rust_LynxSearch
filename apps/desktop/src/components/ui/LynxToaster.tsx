import { Toaster as SonnerToaster } from 'sonner';

export interface LynxToasterProps {
  className?: string;
}

export function LynxToaster({ className }: LynxToasterProps = {}) {
  return (
    <SonnerToaster
      position="bottom-right"
      theme="dark"
      richColors
      closeButton
      className={className}
      toastOptions={{
        className: 'bg-[#0a0a0a] border border-[#262626] text-neutral-100 font-sans shadow-2xl',
      }}
    />
  );
}
