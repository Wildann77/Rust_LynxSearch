import { describe, it, expect } from 'vitest';
import { computeFullPath } from '../lib/path';

describe('computeFullPath', () => {
  it('combines Unix paths correctly', () => {
    expect(computeFullPath('/home/user/project', 'src/main.rs')).toBe(
      '/home/user/project/src/main.rs',
    );
    expect(computeFullPath('/home/user/project/', 'src/main.rs')).toBe(
      '/home/user/project/src/main.rs',
    );
  });

  it('combines Windows paths correctly with backslash normalization', () => {
    expect(computeFullPath('C:\\Users\\dev\\project', 'src/main.rs')).toBe(
      'C:\\Users\\dev\\project\\src\\main.rs',
    );
    expect(computeFullPath('D:\\workspace\\', 'docs\\readme.md')).toBe(
      'D:\\workspace\\docs\\readme.md',
    );
  });

  it('handles empty root or relative path edge cases', () => {
    expect(computeFullPath('', 'docs/readme.md')).toBe('docs/readme.md');
    expect(computeFullPath('/var/data', '')).toBe('/var/data');
  });
});
