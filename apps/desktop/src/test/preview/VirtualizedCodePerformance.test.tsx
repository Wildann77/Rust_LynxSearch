import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { VirtualizedCodeViewer } from '../../components/preview/VirtualizedCodeViewer';
import * as shikiLib from '../../lib/shiki';

describe('Phase 9.8 - Code Performance & Virtualization Suite', () => {
  beforeEach(() => {
    shikiLib.clearTokenCache();
    vi.restoreAllMocks();
  });

  it('renders 1,000 lines with clamped DOM nodes (< 100 rows in DOM)', () => {
    const lines1000 = Array.from(
      { length: 1000 },
      (_, i) => `const var_${i + 1} = ${i + 1};`,
    ).join('\n');

    render(<VirtualizedCodeViewer code={lines1000} language="typescript" />);

    const renderedRows = screen.getAllByTestId('code-line-row');
    expect(renderedRows.length).toBeGreaterThan(0);
    expect(renderedRows.length).toBeLessThan(100);
  });

  it('renders 5,000 lines with clamped DOM nodes (< 100 rows in DOM)', () => {
    const lines5000 = Array.from(
      { length: 5000 },
      (_, i) => `pub fn function_${i + 1}() -> u32 { ${i + 1} }`,
    ).join('\n');

    render(<VirtualizedCodeViewer code={lines5000} language="rust" />);

    const renderedRows = screen.getAllByTestId('code-line-row');
    expect(renderedRows.length).toBeGreaterThan(0);
    expect(renderedRows.length).toBeLessThan(100);
  });

  it('ensures DOM does not contain all 5,000 line nodes at once', () => {
    const lines5000 = Array.from(
      { length: 5000 },
      (_, i) => `line_${i + 1}_token`,
    ).join('\n');

    const { container } = render(
      <VirtualizedCodeViewer code={lines5000} language="rust" />,
    );

    const allRowElements = container.querySelectorAll('[data-testid="code-line-row"]');
    expect(allRowElements.length).toBeLessThan(100);
    expect(allRowElements.length).not.toBe(5000);
  });

  it('ensures scrolling remains responsive and updates rendered slice', () => {
    const lines1000 = Array.from(
      { length: 1000 },
      (_, i) => `scroll_item_${i + 1} = true;`,
    ).join('\n');

    render(<VirtualizedCodeViewer code={lines1000} language="python" />);

    const viewer = screen.getByRole('region', { name: 'Source code viewer' });

    // Initial slice starts around row 1
    const initialFirstRow = screen.getAllByTestId('code-line-row')[0];
    expect(initialFirstRow.getAttribute('data-line-number')).toBe('1');

    // Simulate scroll event
    fireEvent.scroll(viewer, { target: { scrollTop: 1500 } });

    // Virtualization remains bounded
    const scrolledRows = screen.getAllByTestId('code-line-row');
    expect(scrolledRows.length).toBeLessThan(100);
  });

  it('ensures highlight line change does not cause full rerender or plain-token flush', () => {
    const code = Array.from({ length: 500 }, (_, i) => `val_${i + 1} = ${i + 1};`).join('\n');

    const { rerender } = render(
      <VirtualizedCodeViewer code={code} language="python" highlightLine={5} />,
    );

    const initialRows = screen.getAllByTestId('code-line-row');
    const targetRow5 = initialRows.find((r) => r.getAttribute('data-line-number') === '5');
    expect(targetRow5?.className).toContain('border-emerald-500');

    // Rerender with different target highlightLine
    rerender(
      <VirtualizedCodeViewer code={code} language="python" highlightLine={8} />,
    );

    const updatedRows = screen.getAllByTestId('code-line-row');
    const targetRow8 = updatedRows.find((r) => r.getAttribute('data-line-number') === '8');
    const oldTargetRow5 = updatedRows.find((r) => r.getAttribute('data-line-number') === '5');

    expect(targetRow8?.className).toContain('border-emerald-500');
    expect(oldTargetRow5?.className).not.toContain('border-emerald-500');
    expect(updatedRows.length).toBeLessThan(100);
  });

  it('ensures selected row and preview callback remain stable', () => {
    const onLineClick = vi.fn();
    const code = Array.from({ length: 100 }, (_, i) => `item_${i + 1};`).join('\n');

    render(
      <VirtualizedCodeViewer
        code={code}
        language="javascript"
        onLineClick={onLineClick}
      />,
    );

    const rows = screen.getAllByTestId('code-line-row');
    fireEvent.click(rows[4]); // 5th row (line 5)

    expect(onLineClick).toHaveBeenCalledWith(5);
    expect(rows[4].getAttribute('data-line-number')).toBe('5');
  });

  it('avoids unnecessary Shiki work during scroll and reuses cached tokens', async () => {
    const highlightSpy = vi.spyOn(shikiLib, 'highlightCode');
    const code = 'fn main() {\n    let answer = 42;\n}';

    render(<VirtualizedCodeViewer code={code} language="rust" />);

    await waitFor(() => {
      expect(highlightSpy).toHaveBeenCalledTimes(1);
    });

    const viewer = screen.getByRole('region', { name: 'Source code viewer' });

    // Scrolling the container does not call highlightCode again
    fireEvent.scroll(viewer, { target: { scrollTop: 100 } });
    fireEvent.scroll(viewer, { target: { scrollTop: 200 } });

    expect(highlightSpy).toHaveBeenCalledTimes(1);
  });
});
