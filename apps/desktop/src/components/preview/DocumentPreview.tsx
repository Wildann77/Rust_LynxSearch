import * as React from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import {
  FileText,
  ExternalLink,
  Copy,
  X,
  AlertCircle,
  RefreshCw,
  Code2,
  FileCode,
} from 'lucide-react';
import { toast } from 'sonner';
import { useDocumentDetailQuery } from '../../hooks/useDocumentQuery';
import { useSearchStore } from '../../stores/searchStore';
import { Button } from '../ui/button';
import { Badge } from '../ui/badge';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/tooltip';
import { Skeleton } from '../ui/skeleton';
import { formatBytes } from '../../lib/format';
import { openFileInEditor, copyTextToClipboard } from '../../lib/desktop-bridge';
import { cn } from '../../lib/utils';

export interface DocumentPreviewProps {
  documentId: string;
  onClose?: () => void;
  className?: string;
}

export function DocumentPreview({ documentId, onClose, className }: DocumentPreviewProps) {
  const rawQuery = useSearchStore((state) => state.rawQuery);

  const { data: doc, isLoading, isError, error, refetch } = useDocumentDetailQuery(documentId);

  // Extract clean search terms (ignoring syntax tokens like type: or tag:)
  const searchTerms = React.useMemo(() => {
    if (!rawQuery.trim()) return [];
    return rawQuery
      .split(/\s+/)
      .map((term) => term.trim())
      .filter((term) => term.length > 0 && !term.includes(':'))
      .map((term) => term.toLowerCase());
  }, [rawQuery]);

  const fullPath = React.useMemo(() => {
    if (!doc) return '';
    const root = doc.folder_root_path;
    const rel = doc.relative_path;
    const isWindows = root.includes('\\') || /^[a-zA-Z]:[/\\]/.test(root);
    const normalizedRel = isWindows ? rel.replace(/\//g, '\\') : rel.replace(/\\/g, '/');
    const separator = isWindows ? '\\' : '/';
    if (root.endsWith('/') || root.endsWith('\\')) {
      return `${root}${normalizedRel}`;
    }
    return `${root}${separator}${normalizedRel}`;
  }, [doc]);

  const handleCopyPath = React.useCallback(async () => {
    if (!fullPath) return;
    try {
      await copyTextToClipboard(fullPath);
      toast.success('Path berkas disalin ke clipboard');
    } catch {
      toast.error('Gagal menyalin path ke clipboard');
    }
  }, [fullPath]);

  const handleCopyContent = React.useCallback(async () => {
    if (!doc?.content) {
      toast.info('Berkas tidak memiliki isi untuk disalin');
      return;
    }
    try {
      await copyTextToClipboard(doc.content);
      toast.success('Isi berkas disalin ke clipboard');
    } catch {
      toast.error('Gagal menyalin isi berkas ke clipboard');
    }
  }, [doc]);

  const handleOpenEditor = React.useCallback(async () => {
    if (!fullPath) return;
    try {
      await openFileInEditor(fullPath);
      toast.info('Membuka berkas di editor sistem...');
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : typeof err === 'string' ? err : 'Akses ditolak atau berkas tidak ditemukan';
      toast.error(`Gagal membuka berkas: ${msg}`);
    }
  }, [fullPath]);

  // Keyboard shortcut listener for active preview: Cmd+O and Cmd+Shift+C
  React.useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      const isCmd = event.metaKey || event.ctrlKey;
      if (isCmd && event.shiftKey && event.key.toLowerCase() === 'c') {
        event.preventDefault();
        void handleCopyContent();
        return;
      }
      if (isCmd && !event.shiftKey && event.key.toLowerCase() === 'o') {
        event.preventDefault();
        void handleOpenEditor();
        return;
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleCopyContent, handleOpenEditor]);

  // Helper renderer for highlighted code lines
  const renderHighlightedLine = React.useCallback(
    (line: string) => {
      if (searchTerms.length === 0 || !line) {
        return line;
      }

      const escapedTerms = searchTerms
        .map((t) => t.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
        .join('|');
      const regex = new RegExp(`(${escapedTerms})`, 'gi');
      const parts = line.split(regex);

      return parts.map((part, idx) => {
        if (searchTerms.includes(part.toLowerCase())) {
          return (
            <mark
              key={idx}
              className="bg-amber-500/25 text-amber-200 font-medium rounded-xs px-0.5"
            >
              {part}
            </mark>
          );
        }
        return part;
      });
    },
    [searchTerms],
  );

  // Micro-state 1: Loading
  if (isLoading) {
    return (
      <div className={cn('flex flex-col h-full bg-card/20 select-none', className)}>
        <div className="flex h-12 items-center justify-between border-b border-border px-3 shrink-0">
          <div className="flex items-center gap-2">
            <Skeleton className="h-4 w-4 rounded" />
            <Skeleton className="h-4 w-32" />
          </div>
          <div className="flex items-center gap-1">
            <Skeleton className="h-6 w-6 rounded" />
            <Skeleton className="h-6 w-6 rounded" />
            <Skeleton className="h-6 w-6 rounded" />
          </div>
        </div>
        <div className="p-4 space-y-3 flex-1 overflow-hidden">
          <Skeleton className="h-6 w-3/4" />
          <Skeleton className="h-4 w-1/2" />
          <div className="pt-4 space-y-2">
            <Skeleton className="h-4 w-full" />
            <Skeleton className="h-4 w-5/6" />
            <Skeleton className="h-4 w-4/6" />
            <Skeleton className="h-4 w-full" />
          </div>
        </div>
      </div>
    );
  }

  // Micro-state 2: Error
  if (isError || !doc) {
    return (
      <div className={cn('flex flex-col h-full bg-card/20 p-6 items-center justify-center text-center', className)}>
        <div className="inline-flex h-12 w-12 items-center justify-center rounded-xl bg-destructive/10 border border-destructive/20 text-destructive mb-3">
          <AlertCircle className="h-6 w-6" />
        </div>
        <h3 className="text-sm font-semibold text-foreground mb-1">Gagal Memuat Dokumen</h3>
        <p className="text-xs text-muted-foreground max-w-xs mb-4">
          {error?.message || 'Dokumen tidak ditemukan atau tidak dapat diakses.'}
        </p>
        <Button variant="outline" size="sm" onClick={() => void refetch()} className="gap-1.5">
          <RefreshCw className="h-3.5 w-3.5" />
          <span>Coba Lagi</span>
        </Button>
      </div>
    );
  }

  const isMarkdown =
    doc.type === 'doc' ||
    doc.relative_path.endsWith('.md') ||
    doc.relative_path.endsWith('.markdown');

  const lines = doc.content.split('\n');

  return (
    <div className={cn('flex flex-col h-full bg-card/20 overflow-hidden', className)}>
      {/* Header Bar */}
      <div className="flex h-12 items-center justify-between border-b border-border px-3 shrink-0 bg-card/40">
        <div className="flex items-center gap-2 min-w-0 pr-2">
          {isMarkdown ? (
            <FileText className="h-4 w-4 text-primary shrink-0" />
          ) : doc.type === 'code' ? (
            <Code2 className="h-4 w-4 text-emerald-400 shrink-0" />
          ) : (
            <FileCode className="h-4 w-4 text-amber-400 shrink-0" />
          )}

          <div className="flex flex-col min-w-0">
            <span className="text-xs font-semibold tracking-tight text-foreground truncate" title={doc.title}>
              {doc.title}
            </span>
            <span
              className="font-mono text-[10px] text-muted-foreground truncate cursor-pointer hover:text-foreground"
              title={fullPath}
              onClick={handleCopyPath}
            >
              {doc.relative_path}
            </span>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center gap-1 shrink-0">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                onClick={handleOpenEditor}
                className="h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer"
                aria-label="Open in Editor (Cmd+O)"
              >
                <ExternalLink className="h-3.5 w-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">Open in Editor (⌘O)</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                onClick={handleCopyContent}
                className="h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer"
                aria-label="Copy File Content (Cmd+Shift+C)"
              >
                <Copy className="h-3.5 w-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">Copy Content (⌘⇧C)</TooltipContent>
          </Tooltip>

          {onClose ? (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={onClose}
                  className="h-7 w-7 text-muted-foreground hover:text-foreground cursor-pointer"
                  aria-label="Close Preview"
                >
                  <X className="h-3.5 w-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">Close Preview (])</TooltipContent>
            </Tooltip>
          ) : null}
        </div>
      </div>

      {/* Metadata Subheader */}
      <div className="flex items-center gap-2 border-b border-border/60 bg-secondary/20 px-3 py-1.5 text-[11px] font-mono text-muted-foreground shrink-0 overflow-x-auto">
        <Badge variant="outline" className="text-[10px] uppercase font-mono px-1.5 py-0 h-4">
          {doc.type}
        </Badge>
        {doc.language ? (
          <Badge variant="secondary" className="text-[10px] font-mono px-1.5 py-0 h-4">
            {doc.language}
          </Badge>
        ) : null}
        <span className="text-border">|</span>
        <span>{formatBytes(doc.file_size)}</span>
        <span className="text-border">|</span>
        <span>{lines.length} baris</span>
      </div>

      {/* Body Content */}
      <div className="flex-1 min-h-0 overflow-y-auto select-text">
        {isMarkdown ? (
          <article className="p-5 max-w-none text-xs leading-relaxed text-foreground select-text font-sans">
            <ReactMarkdown
              remarkPlugins={[remarkGfm]}
              components={{
                h1: ({ children }) => (
                  <h1 className="text-lg font-bold tracking-tight text-foreground mt-4 mb-2 border-b border-border pb-1">
                    {children}
                  </h1>
                ),
                h2: ({ children }) => (
                  <h2 className="text-base font-semibold tracking-tight text-foreground mt-3 mb-1.5">
                    {children}
                  </h2>
                ),
                h3: ({ children }) => (
                  <h3 className="text-sm font-semibold text-foreground mt-2.5 mb-1">
                    {children}
                  </h3>
                ),
                p: ({ children }) => (
                  <p className="my-2 leading-relaxed text-neutral-300">
                    {children}
                  </p>
                ),
                ul: ({ children }) => (
                  <ul className="list-disc list-inside my-2 space-y-1 text-neutral-300">
                    {children}
                  </ul>
                ),
                ol: ({ children }) => (
                  <ol className="list-decimal list-inside my-2 space-y-1 text-neutral-300">
                    {children}
                  </ol>
                ),
                li: ({ children }) => <li className="leading-relaxed">{children}</li>,
                blockquote: ({ children }) => (
                  <blockquote className="border-l-2 border-primary/70 pl-3 italic text-neutral-400 my-2">
                    {children}
                  </blockquote>
                ),
                code: ({ className: codeClassName, children, ...props }) => {
                  const match = /language-(\w+)/.exec(codeClassName || '');
                  const isInline = !match && !String(children).includes('\n');
                  if (isInline) {
                    return (
                      <code className="rounded bg-secondary/80 px-1.5 py-0.5 font-mono text-[11px] text-foreground" {...props}>
                        {children}
                      </code>
                    );
                  }
                  return (
                    <code className={cn('font-mono text-[11px]', codeClassName)} {...props}>
                      {children}
                    </code>
                  );
                },
                pre: ({ children }) => (
                  <pre className="rounded-md border border-border bg-black/60 p-3 font-mono text-xs overflow-x-auto my-3 text-neutral-200">
                    {children}
                  </pre>
                ),
                table: ({ children }) => (
                  <div className="overflow-x-auto my-3 border border-border rounded-md">
                    <table className="w-full border-collapse text-xs text-left">{children}</table>
                  </div>
                ),
                th: ({ children }) => (
                  <th className="border-b border-border bg-secondary/50 px-2.5 py-1.5 font-semibold text-foreground">
                    {children}
                  </th>
                ),
                td: ({ children }) => (
                  <td className="border-b border-border/40 px-2.5 py-1 text-neutral-300">
                    {children}
                  </td>
                ),
                a: ({ href, children }) => (
                  <a
                    href={href}
                    target="_blank"
                    rel="noreferrer"
                    className="text-primary underline underline-offset-2 hover:text-primary/80"
                  >
                    {children}
                  </a>
                ),
              }}
            >
              {doc.content}
            </ReactMarkdown>
          </article>
        ) : (
          /* Plain / Source Code Viewer with Line Numbers (Phase 7.11 placeholder) */
          <div className="flex font-mono text-xs leading-relaxed min-h-full select-text bg-black/40">
            {/* Gutter: Line Numbers */}
            <div className="w-12 shrink-0 py-3 pr-2.5 text-right text-neutral-500 border-r border-border/50 bg-neutral-950/40 select-none font-mono">
              {lines.map((_, i) => (
                <div key={i} className="h-5 leading-5 text-[11px]">
                  {i + 1}
                </div>
              ))}
            </div>

            {/* Code Content */}
            <div className="flex-1 py-3 pl-3 overflow-x-auto">
              {lines.map((line, i) => (
                <div key={i} className="h-5 leading-5 whitespace-pre font-mono text-[11px] text-neutral-200">
                  {renderHighlightedLine(line)}
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
