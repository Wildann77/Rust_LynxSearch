/**
 * Compute absolute path safely across Windows, Linux, and macOS.
 */
export function computeFullPath(folderRootPath: string, relativePath: string): string {
  if (!folderRootPath) return relativePath;
  if (!relativePath) return folderRootPath;

  const isWindows = folderRootPath.includes('\\') || /^[a-zA-Z]:[/\\]/.test(folderRootPath);
  const normalizedRel = isWindows ? relativePath.replace(/\//g, '\\') : relativePath.replace(/\\/g, '/');
  const separator = isWindows ? '\\' : '/';

  if (folderRootPath.endsWith('/') || folderRootPath.endsWith('\\')) {
    return `${folderRootPath}${normalizedRel}`;
  }
  return `${folderRootPath}${separator}${normalizedRel}`;
}
