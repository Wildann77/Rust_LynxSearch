import React from 'react';
import { describe, it, expect, vi, beforeAll, afterEach, afterAll } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { http, HttpResponse } from 'msw';
import { server } from './mocks/server';
import { ResultList } from '../components/search/ResultList';
import { HealthDot } from '../components/common/HealthDot';
import { TooltipProvider } from '../components/ui/tooltip';
import type { SearchResultItem } from '../types/search';

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: Infinity,
      },
    },
  });
}

function renderWithProviders(ui: React.ReactElement, client = createTestQueryClient()) {
  return render(
    <QueryClientProvider client={client}>
      <TooltipProvider>{ui}</TooltipProvider>
    </QueryClientProvider>,
  );
}

const mockItem: SearchResultItem = {
  id: 'a0000000-0000-4000-8000-000000000001',
  title: 'Test Document',
  relative_path: 'docs/test.md',
  type: 'doc',
  language: null,
  project: 'lynxsearch',
  tags: ['test'],
  highlights: [],
  score: 1.0,
  file_size: 1024,
  updated_at: new Date().toISOString(),
};

describe('Four Micro-State Tests (Phase 10.9)', () => {
  beforeAll(() => server.listen({ onUnhandledRequest: 'bypass' }));
  afterEach(() => server.resetHandlers());
  afterAll(() => server.close());
  describe('1. ResultList Micro-States', () => {
    it('renders Loading state with skeletons', () => {
      const { container } = renderWithProviders(
        <ResultList
          items={[]}
          total={0}
          page={1}
          pageSize={20}
          onPageChange={vi.fn()}
          selectedId={null}
          onSelectItem={vi.fn()}
          isLoading={true}
        />,
      );
      expect(container.querySelectorAll('.animate-pulse').length).toBeGreaterThan(0);
    });

    it('renders Empty state when items is empty', () => {
      renderWithProviders(
        <ResultList
          items={[]}
          total={0}
          page={1}
          pageSize={20}
          onPageChange={vi.fn()}
          selectedId={null}
          onSelectItem={vi.fn()}
          isLoading={false}
          isSearching={true}
        />,
      );
      expect(screen.getByText(/Tidak ada dokumen yang cocok/i)).toBeDefined();
    });

    it('renders Error state with message and retry button', () => {
      const onRetry = vi.fn();
      renderWithProviders(
        <ResultList
          items={[]}
          total={0}
          page={1}
          pageSize={20}
          onPageChange={vi.fn()}
          selectedId={null}
          onSelectItem={vi.fn()}
          isError={true}
          errorMessage="Koneksi terputus ke Elasticsearch"
          onRetry={onRetry}
        />,
      );
      expect(screen.getByText('Koneksi terputus ke Elasticsearch')).toBeDefined();
      const retryBtn = screen.getByRole('button', { name: /Coba Lagi/i });
      fireEvent.click(retryBtn);
      expect(onRetry).toHaveBeenCalledTimes(1);
    });

    it('renders Success state with results list', () => {
      const onSelect = vi.fn();
      renderWithProviders(
        <ResultList
          items={[mockItem]}
          total={1}
          page={1}
          pageSize={20}
          onPageChange={vi.fn()}
          selectedId={null}
          onSelectItem={onSelect}
          isLoading={false}
        />,
      );
      expect(screen.getByText('Test Document')).toBeDefined();
      expect(screen.getByText('docs/test.md')).toBeDefined();
    });
  });

  describe('2. HealthDot Micro-States', () => {
    it('renders Offline / Error state when backend is unreachable', async () => {
      server.use(
        http.get('*/api/health', () => {
          return HttpResponse.error();
        }),
      );

      const client = createTestQueryClient();
      renderWithProviders(<HealthDot showLabel={true} retry={false} />, client);

      await waitFor(() => {
        expect(screen.getByText('Offline')).toBeDefined();
      });
    });

    it('renders Ready / Success state when backend is healthy', async () => {
      const client = createTestQueryClient();
      renderWithProviders(<HealthDot showLabel={true} />, client);

      await waitFor(() => {
        expect(screen.getByText('Ready')).toBeDefined();
      });
    });
  });
});
