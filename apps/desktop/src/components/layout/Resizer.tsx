import * as React from 'react';
import { useUIStore } from '../../stores/uiStore';
import { cn } from '../../lib/utils';

export interface ResizerProps {
  className?: string;
}

export function Resizer({ className }: ResizerProps) {
  const [isDragging, setIsDragging] = React.useState(false);
  const setPreviewWidth = useUIStore((state) => state.setPreviewWidth);

  const handlePointerDown = React.useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      e.preventDefault();
      setIsDragging(true);

      const handlePointerMove = (moveEvent: PointerEvent) => {
        // Preview pane is positioned on the right side of the screen
        const newWidth = window.innerWidth - moveEvent.clientX;
        setPreviewWidth(newWidth);
      };

      const handlePointerUp = () => {
        setIsDragging(false);
        window.removeEventListener('pointermove', handlePointerMove);
        window.removeEventListener('pointerup', handlePointerUp);
      };

      window.addEventListener('pointermove', handlePointerMove);
      window.addEventListener('pointerup', handlePointerUp);
    },
    [setPreviewWidth],
  );

  return (
    <div
      role="separator"
      aria-orientation="vertical"
      tabIndex={0}
      onPointerDown={handlePointerDown}
      className={cn(
        'relative group w-[3px] shrink-0 cursor-col-resize bg-border hover:bg-primary/70 transition-colors select-none z-10',
        isDragging && 'bg-primary ring-1 ring-primary/50',
        className,
      )}
      title="Drag to resize preview pane (420px - 640px)"
    >
      {/* Expanded invisible hitbox for easy grabbing */}
      <div className="absolute -left-1.5 -right-1.5 top-0 bottom-0 z-20 cursor-col-resize" />
    </div>
  );
}
