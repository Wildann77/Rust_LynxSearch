import { describe, it, expect } from 'vitest';

describe('Frontend environment smoke test', () => {
  it('renders test environment with happy-dom', () => {
    const div = document.createElement('div');
    div.textContent = 'LynxSearch Ready';
    document.body.appendChild(div);

    expect(document.body.textContent).toContain('LynxSearch Ready');
  });
});
