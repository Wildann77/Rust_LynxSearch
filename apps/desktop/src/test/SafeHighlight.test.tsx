import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SafeHighlight, parseHighlightMarkers } from '@/components/search/SafeHighlight';

describe('parseHighlightMarkers', () => {
  it('returns empty array for empty or undefined-like strings', () => {
    expect(parseHighlightMarkers('')).toEqual([]);
  });

  it('parses plain text with no markers', () => {
    expect(parseHighlightMarkers('plain text only')).toEqual([
      { text: 'plain text only', isMatch: false },
    ]);
  });

  it('parses single marker with surrounding text', () => {
    expect(parseHighlightMarkers('prefix <em>match</em> suffix')).toEqual([
      { text: 'prefix ', isMatch: false },
      { text: 'match', isMatch: true },
      { text: ' suffix', isMatch: false },
    ]);
  });

  it('parses multiple markers across multiline text', () => {
    const snippet = 'line 1: <em>first</em>\nline 2: <em>second</em>';
    expect(parseHighlightMarkers(snippet)).toEqual([
      { text: 'line 1: ', isMatch: false },
      { text: 'first', isMatch: true },
      { text: '\nline 2: ', isMatch: false },
      { text: 'second', isMatch: true },
    ]);
  });

  it('handles unclosed or malformed tags gracefully as text', () => {
    expect(parseHighlightMarkers('broken <em>tag without close')).toEqual([
      { text: 'broken <em>tag without close', isMatch: false },
    ]);
    expect(parseHighlightMarkers('stray </em> tag')).toEqual([
      { text: 'stray </em> tag', isMatch: false },
    ]);
  });
});

describe('SafeHighlight Component', () => {
  it('renders nothing when snippet is empty', () => {
    const { container } = render(<SafeHighlight snippet="" />);
    expect(container.firstChild).toBeNull();
  });

  it('renders semantic <mark> for matched text with default dark minimalist classes', () => {
    render(<SafeHighlight snippet="pub fn <em>load_config</em>() -> Result" />);

    const mark = screen.getByText('load_config');
    expect(mark.tagName.toLowerCase()).toBe('mark');
    expect(mark.className).toContain('bg-amber-500/20');
    expect(mark.className).toContain('text-amber-200');
    expect(mark.className).toContain('rounded-xs');
  });

  it('allows customizing markClassName and container element via as prop', () => {
    render(
      <SafeHighlight
        as="p"
        snippet="Searching <em>keyword</em> in docs"
        className="custom-container"
        markClassName="custom-mark"
      />
    );

    const container = screen.getByText(/Searching/).closest('p');
    expect(container).not.toBeNull();
    expect(container?.className).toContain('custom-container');

    const mark = screen.getByText('keyword');
    expect(mark.className).toContain('custom-mark');
  });

  it('does NOT execute or inject HTML markup as DOM nodes (XSS immunity)', () => {
    const maliciousSnippet =
      'Look at this: <script>alert("pwned")</script> and <img src="x" onerror="console.error(1)" /> with <em>safe match</em>';

    const { container } = render(<SafeHighlight snippet={maliciousSnippet} />);

    // Proves no script or img elements exist in the rendered DOM tree
    expect(container.querySelector('script')).toBeNull();
    expect(container.querySelector('img')).toBeNull();

    // Proves text is rendered literally without evaluation
    expect(container.textContent).toContain('<script>alert("pwned")</script>');
    expect(container.textContent).toContain('<img src="x" onerror="console.error(1)" />');

    // Only semantic <mark> exists inside the container
    const marks = container.querySelectorAll('mark');
    expect(marks).toHaveLength(1);
    expect(marks[0].textContent).toBe('safe match');
  });

  it('renders malicious HTML inside the highlight tag literally as text', () => {
    const snippetWithEvilTag = 'Found <em><svg onload="evil()"></em> in file';

    const { container } = render(<SafeHighlight snippet={snippetWithEvilTag} />);

    expect(container.querySelector('svg')).toBeNull();
    const mark = container.querySelector('mark');
    expect(mark).not.toBeNull();
    expect(mark?.textContent).toBe('<svg onload="evil()">');
  });
});
