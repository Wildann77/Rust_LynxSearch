import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { VirtualizedCodeViewer } from '../../components/preview/VirtualizedCodeViewer';

describe('VirtualizedCodeViewer (Phase 9.6)', () => {
  it('renders line numbers and code text', async () => {
    const code = 'fn main() {\n    let val = 42;\n    println!("{}", val);\n}';
    render(<VirtualizedCodeViewer code={code} language="rust" />);

    // Line numbers in gutter
    expect(screen.getByText('1')).toBeDefined();
    expect(screen.getByText('2')).toBeDefined();
    expect(screen.getByText('3')).toBeDefined();
    expect(screen.getByText('4')).toBeDefined();

    // Code lines exist
    await waitFor(() => {
      expect(screen.getByText(/main/)).toBeDefined();
    });
  });

  it('preserves horizontal whitespace and indentation', () => {
    const code = 'line1\n    indented 4 spaces\n\t\ttabbed';
    render(<VirtualizedCodeViewer code={code} />);

    expect(screen.getByText('indented 4 spaces')).toBeDefined();
  });

  it('highlights search terms inside code with <mark> tags', async () => {
    const code = 'const authenticateUser = () => {\n  return true;\n};';
    render(
      <VirtualizedCodeViewer
        code={code}
        language="typescript"
        searchTerms={['authenticateUser']}
      />,
    );

    await waitFor(() => {
      const marks = screen.getAllByText('authenticateUser');
      const markTag = marks.find((el) => el.tagName.toLowerCase() === 'mark');
      expect(markTag).toBeDefined();
    });
  });

  it('highlights the active target line with distinct border and accent styling', () => {
    const code = 'line 1\nline 2 (target)\nline 3';
    render(<VirtualizedCodeViewer code={code} highlightLine={2} />);

    const rows = screen.getAllByTestId('code-line-row');
    const targetRow = rows.find((r) => r.getAttribute('data-line-number') === '2');
    expect(targetRow).toBeDefined();
    expect(targetRow?.className).toContain('border-emerald-500');
  });

  it('renders accessible region with role and label', () => {
    const code = 'let x = 1;';
    render(<VirtualizedCodeViewer code={code} />);

    const viewerRegion = screen.getByRole('region', { name: 'Source code viewer' });
    expect(viewerRegion).toBeDefined();
    expect(viewerRegion.getAttribute('tabindex')).toBe('0');
  });

  it('triggers onLineClick callback when a line row is clicked', () => {
    const onLineClick = vi.fn();
    const code = 'line 1\nline 2\nline 3';
    render(<VirtualizedCodeViewer code={code} onLineClick={onLineClick} />);

    const rows = screen.getAllByTestId('code-line-row');
    fireEvent.click(rows[1]);

    expect(onLineClick).toHaveBeenCalledWith(2);
  });

  it('virtualizes large documents (5,000 lines) rendering only viewport + overscan rows', () => {
    // Generate 5,000 lines
    const largeLines = Array.from({ length: 5000 }, (_, i) => `let item_${i + 1} = ${i + 1};`);
    const largeCode = largeLines.join('\n');

    render(<VirtualizedCodeViewer code={largeCode} language="rust" />);

    const renderedRows = screen.getAllByTestId('code-line-row');
    // Ensure all 5,000 lines are NOT rendered in DOM at once
    expect(renderedRows.length).toBeLessThan(100);
    expect(renderedRows.length).toBeGreaterThan(0);
  });

  it('handles invalid line numbers (-5, 999999) gracefully without crashing', () => {
    const code = 'line 1\nline 2\nline 3';
    expect(() => {
      render(<VirtualizedCodeViewer code={code} highlightLine={-5} />);
    }).not.toThrow();

    expect(() => {
      render(<VirtualizedCodeViewer code={code} highlightLine={999999} />);
    }).not.toThrow();
  });
});
