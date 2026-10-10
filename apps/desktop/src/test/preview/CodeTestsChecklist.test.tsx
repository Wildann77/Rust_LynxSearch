import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { VirtualizedCodeViewer } from '../../components/preview/VirtualizedCodeViewer';
import * as shikiLib from '../../lib/shiki';

describe('Phase 9.9 - Code Tests Checklist (Frontend Scope)', () => {
  beforeEach(() => {
    shikiLib.clearTokenCache();
    vi.restoreAllMocks();
  });

  it('verifies virtualization row count is strictly bounded on large files', () => {
    const lines = Array.from({ length: 3000 }, (_, i) => `let row_${i + 1} = ${i + 1};`).join(
      '\n',
    );

    render(<VirtualizedCodeViewer code={lines} language="rust" />);

    const rows = screen.getAllByTestId('code-line-row');
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.length).toBeLessThan(100);
  });

  it('verifies auto-scroll targets and highlights active line number', () => {
    const lines = Array.from(
      { length: 200 },
      (_, i) => `function step_${i + 1}() { return ${i + 1}; }`,
    ).join('\n');

    render(
      <VirtualizedCodeViewer
        code={lines}
        language="javascript"
        highlightLine={3}
      />,
    );

    const rows = screen.getAllByTestId('code-line-row');
    const targetRow = rows.find((r) => r.getAttribute('data-line-number') === '3');

    expect(targetRow).toBeDefined();
    expect(targetRow?.className).toContain('border-emerald-500');
    expect(targetRow?.className).toContain('bg-emerald-500/15');
  });

  it('verifies unsupported language fallback gracefully renders plain text lines', async () => {
    const code = 'UNKNOWN_SYNTAX {\n  custom_instruction: 99;\n}';

    render(
      <VirtualizedCodeViewer
        code={code}
        language="some_completely_unsupported_language"
      />,
    );

    // Gutter numbers are present
    expect(screen.getByText('1')).toBeDefined();
    expect(screen.getByText('2')).toBeDefined();
    expect(screen.getByText('3')).toBeDefined();

    // Plain text content rendered safely
    await waitFor(() => {
      expect(screen.getByText(/UNKNOWN_SYNTAX/)).toBeDefined();
      expect(screen.getByText(/custom_instruction/)).toBeDefined();
    });
  });

  it('verifies search term highlight for camelCase and snake_case identifiers', async () => {
    const code = [
      'function authenticateUser() { return true; }',
      'fn authenticate_user() -> bool { true }',
    ].join('\n');

    render(
      <VirtualizedCodeViewer
        code={code}
        language="typescript"
        searchTerms={['authenticateUser', 'authenticate_user']}
      />,
    );

    await waitFor(() => {
      const mark1 = screen.getAllByText('authenticateUser').find((el) => el.tagName.toLowerCase() === 'mark');
      const mark2 = screen.getAllByText('authenticate_user').find((el) => el.tagName.toLowerCase() === 'mark');
      expect(mark1).toBeDefined();
      expect(mark2).toBeDefined();
    });
  });
});
