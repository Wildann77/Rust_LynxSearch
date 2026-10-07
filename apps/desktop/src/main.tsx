import React from 'react';
import ReactDOM from 'react-dom/client';
import { QueryClientProvider } from '@tanstack/react-query';
import { queryClient } from './lib/query-client';
import App from './App';
import './index.css';

import { focusWindow, setDesktopZoom } from './lib/desktop-bridge';

// Window Focus & Touchpad Gesture Control
if (typeof window !== 'undefined') {
  // 1. Wheel zoom (trackpad pinch synthesizes wheel event with ctrlKey=true)
  const preventWheelZoom = (e: WheelEvent) => {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
    }
  };
  window.addEventListener('wheel', preventWheelZoom, { passive: false, capture: true });
  document.addEventListener('wheel', preventWheelZoom, { passive: false, capture: true });

  // 2. Multi-touch pinch (trackpad or touchscreen pinch)
  const preventMultiTouch = (e: TouchEvent) => {
    if (e.touches.length > 1) {
      e.preventDefault();
    }
  };
  window.addEventListener('touchstart', preventMultiTouch, { passive: false, capture: true });
  window.addEventListener('touchmove', preventMultiTouch, { passive: false, capture: true });
  document.addEventListener('touchstart', preventMultiTouch, { passive: false, capture: true });
  document.addEventListener('touchmove', preventMultiTouch, { passive: false, capture: true });

  // 3. WebKit gesture events
  const preventGesture = (e: Event) => e.preventDefault();
  window.addEventListener('gesturestart', preventGesture, { capture: true });
  window.addEventListener('gesturechange', preventGesture, { capture: true });
  window.addEventListener('gestureend', preventGesture, { capture: true });
  document.addEventListener('gesturestart', preventGesture, { capture: true });
  document.addEventListener('gesturechange', preventGesture, { capture: true });
  document.addEventListener('gestureend', preventGesture, { capture: true });

  // 4. Click window to bring to front (focus)
  window.addEventListener('mousedown', () => {
    void focusWindow();
  }, { capture: true });

  // 5. Keyboard zoom (Ctrl + + / Ctrl + - / Ctrl + 0)
  let currentZoom = 1.0;
  window.addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.ctrlKey || e.metaKey) {
      if (e.key === '=' || e.key === '+' || e.code === 'Equal' || e.code === 'NumpadAdd') {
        e.preventDefault();
        currentZoom = Math.min(2.0, +(currentZoom + 0.1).toFixed(2));
        void setDesktopZoom(currentZoom);
      } else if (e.key === '-' || e.code === 'Minus' || e.code === 'NumpadSubtract') {
        e.preventDefault();
        currentZoom = Math.max(0.7, +(currentZoom - 0.1).toFixed(2));
        void setDesktopZoom(currentZoom);
      } else if (e.key === '0' || e.code === 'Digit0' || e.code === 'Numpad0') {
        e.preventDefault();
        currentZoom = 1.0;
        void setDesktopZoom(currentZoom);
      }
    }
  }, { capture: true });
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>,
);
