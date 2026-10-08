import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import * as fs from 'fs';
import * as path from 'path';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { Slider } from '../components/ui/slider';
import { SearchBar } from '../components/search/SearchBar';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

describe('Accessibility Requirements (Task 7.22)', () => {
  it('defines prefers-reduced-motion media query in index.css', () => {
    const cssPath = path.resolve(__dirname, '../index.css');
    const cssContent = fs.readFileSync(cssPath, 'utf-8');

    expect(cssContent).toContain('@media (prefers-reduced-motion: reduce)');
    expect(cssContent).toContain('animation-duration: 0.01ms !important');
    expect(cssContent).toContain('transition-duration: 0.01ms !important');
  });

  it('defines global visible focus ring in index.css', () => {
    const cssPath = path.resolve(__dirname, '../index.css');
    const cssContent = fs.readFileSync(cssPath, 'utf-8');

    expect(cssContent).toContain(':focus-visible');
    expect(cssContent).toContain('outline: 2px solid');
  });

  it('renders Button with visible focus-visible ring classes', () => {
    render(<Button>Click me</Button>);
    const button = screen.getByRole('button', { name: 'Click me' });

    expect(button.className).toContain('focus-visible:ring-2');
    expect(button.className).toContain('focus-visible:outline-none');
  });

  it('renders Input with visible focus-visible ring classes', () => {
    render(<Input placeholder="Type here" aria-label="Input field" />);
    const input = screen.getByRole('textbox', { name: 'Input field' });

    expect(input.className).toContain('focus-visible:ring-2');
  });

  it('renders Slider with aria-label and visible focus-visible ring classes on Thumb', () => {
    render(<Slider value={[5]} min={0} max={10} aria-label="Bobot Relevansi" />);
    const sliderThumb = screen.getByRole('slider', { name: 'Bobot Relevansi' });

    expect(sliderThumb).toBeDefined();
    expect(sliderThumb.getAttribute('aria-label')).toBe('Bobot Relevansi');
    expect(sliderThumb.className).toContain('focus-visible:ring-2');
  });

  it('renders SearchBar with accessible aria-label on input', () => {
    const queryClient = new QueryClient();
    render(
      <QueryClientProvider client={queryClient}>
        <SearchBar />
      </QueryClientProvider>,
    );

    const input = screen.getByLabelText('Pencarian dokumen, kode, dan tag');
    expect(input).toBeDefined();
  });
});
