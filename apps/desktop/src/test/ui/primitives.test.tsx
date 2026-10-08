import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Dialog,
  DialogTrigger,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';
import { Popover, PopoverTrigger, PopoverContent } from '@/components/ui/popover';
import {
  TooltipProvider,
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from '@/components/ui/tooltip';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { Alert, AlertTitle, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { Slider } from '@/components/ui/slider';
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from '@/components/ui/select';

describe('shadcn/ui Foundation Primitives', () => {
  it('renders Button with variants and sizes', () => {
    const { rerender } = render(<Button>Click me</Button>);
    const button = screen.getByRole('button', { name: 'Click me' });
    expect(button).toBeDefined();
    expect(button.className).toContain('bg-primary');

    rerender(<Button variant="destructive">Delete</Button>);
    expect(screen.getByRole('button', { name: 'Delete' }).className).toContain(
      'bg-destructive',
    );

    rerender(<Button variant="outline" size="sm">Small</Button>);
    const smallBtn = screen.getByRole('button', { name: 'Small' });
    expect(smallBtn.className).toContain('border');
    expect(smallBtn.className).toContain('h-8');
  });

  it('renders Input with proper styles and disabled state', () => {
    render(<Input placeholder="Search..." disabled />);
    const input = screen.getByPlaceholderText('Search...') as HTMLInputElement;
    expect(input.disabled).toBe(true);
    expect(input.className).toContain('border-border');
    expect(input.className).toContain('bg-card');
  });

  it('renders Dialog structure', () => {
    render(
      <Dialog open={true}>
        <DialogTrigger asChild>
          <Button>Open</Button>
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Dialog Title</DialogTitle>
            <DialogDescription>Dialog Description</DialogDescription>
          </DialogHeader>
        </DialogContent>
      </Dialog>,
    );
    expect(screen.getByText('Dialog Title')).toBeDefined();
    expect(screen.getByText('Dialog Description')).toBeDefined();
  });

  it('renders Popover content when open', () => {
    render(
      <Popover open={true}>
        <PopoverTrigger asChild>
          <Button>Popover</Button>
        </PopoverTrigger>
        <PopoverContent>Popover Body</PopoverContent>
      </Popover>,
    );
    expect(screen.getByText('Popover Body')).toBeDefined();
  });

  it('renders Tooltip', () => {
    render(
      <TooltipProvider>
        <Tooltip open={true}>
          <TooltipTrigger asChild>
            <Button>Hover</Button>
          </TooltipTrigger>
          <TooltipContent>Tooltip Info</TooltipContent>
        </Tooltip>
      </TooltipProvider>,
    );
    expect(screen.getByText('Tooltip Info')).toBeDefined();
  });

  it('renders ScrollArea with viewport', () => {
    const { container } = render(
      <ScrollArea className="h-40 w-40">
        <div>Scrollable Content</div>
      </ScrollArea>,
    );
    expect(container.querySelector('[data-radix-scroll-area-viewport]')).not.toBeNull();
    expect(screen.getByText('Scrollable Content')).toBeDefined();
  });

  it('renders Skeleton with pulse class', () => {
    const { container } = render(<Skeleton className="h-6 w-24" />);
    const skeleton = container.firstChild as HTMLElement;
    expect(skeleton.className).toContain('animate-pulse');
    expect(skeleton.className).toContain('bg-muted/50');
  });

  it('renders Alert with default and destructive variant', () => {
    const { rerender } = render(
      <Alert>
        <AlertTitle>Notice</AlertTitle>
        <AlertDescription>System normal.</AlertDescription>
      </Alert>,
    );
    expect(screen.getByText('Notice')).toBeDefined();
    expect(screen.getByText('System normal.')).toBeDefined();

    rerender(
      <Alert variant="destructive">
        <AlertTitle>Error</AlertTitle>
        <AlertDescription>Failed.</AlertDescription>
      </Alert>,
    );
    expect(screen.getByRole('alert').className).toContain('border-destructive');
  });

  it('renders Badge with variants', () => {
    const { rerender } = render(<Badge>Default</Badge>);
    expect(screen.getByText('Default').className).toContain('bg-primary');

    rerender(<Badge variant="secondary">Secondary</Badge>);
    expect(screen.getByText('Secondary').className).toContain('bg-secondary');

    rerender(<Badge variant="accent">Accent</Badge>);
    expect(screen.getByText('Accent').className).toContain('text-primary');
  });

  it('renders Checkbox with states', () => {
    render(<Checkbox checked={true} aria-label="agree" />);
    const checkbox = screen.getByRole('checkbox', { name: 'agree' });
    expect(checkbox.getAttribute('data-state')).toBe('checked');
  });

  it('renders Slider', () => {
    const { container } = render(<Slider defaultValue={[50]} max={100} step={1} />);
    expect(container.querySelector('[role="slider"]')).not.toBeNull();
  });

  it('renders Select trigger and value', () => {
    render(
      <Select defaultValue="rust">
        <SelectTrigger aria-label="language">
          <SelectValue placeholder="Select language" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="rust">Rust</SelectItem>
          <SelectItem value="ts">TypeScript</SelectItem>
        </SelectContent>
      </Select>,
    );
    expect(screen.getByRole('combobox', { name: 'language' })).toBeDefined();
    expect(screen.getByText('Rust')).toBeDefined();
  });
});
