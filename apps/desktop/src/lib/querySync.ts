export type FilterKey = 'extension' | 'type' | 'language' | 'tag' | 'project';

export interface ActiveFilters {
  extension?: string;
  type?: string;
  language?: string;
  tag?: string;
  project?: string;
}

const FILTER_ALIASES: Record<string, FilterKey> = {
  extension: 'extension',
  ext: 'extension',
  type: 'type',
  language: 'language',
  lang: 'language',
  tag: 'tag',
  tags: 'tag',
  project: 'project',
};

const VALID_KEYS: FilterKey[] = ['extension', 'type', 'language', 'tag', 'project'];

interface ParsedToken {
  type: 'free' | 'filter';
  key?: FilterKey;
  values?: string[];
  raw: string;
}

/**
 * Tokenize input query while respecting quoted strings.
 */
function tokenizeQuery(input: string): string[] {
  const tokens: string[] = [];
  let i = 0;
  const len = input.length;

  while (i < len) {
    while (i < len && /\s/.test(input[i])) {
      i++;
    }
    if (i >= len) break;

    const start = i;

    // Check if token starts with a quote
    if (input[i] === '"') {
      i++;
      while (i < len && input[i] !== '"') {
        if (input[i] === '\\' && i + 1 < len) {
          i += 2;
        } else {
          i++;
        }
      }
      if (i < len && input[i] === '"') {
        i++;
      }
      tokens.push(input.slice(start, i));
      continue;
    }

    // Read until whitespace, but handle key:"quoted val"
    let inQuotes = false;
    while (i < len && (!/\s/.test(input[i]) || inQuotes)) {
      if (input[i] === '"' && (i === 0 || input[i - 1] !== '\\')) {
        inQuotes = !inQuotes;
      }
      i++;
    }
    tokens.push(input.slice(start, i));
  }

  return tokens;
}

/**
 * Parse an individual token to see if it is a filter key:value pair.
 */
function parseToken(token: string): ParsedToken {
  // Avoid matching URL schemes like http:// or Rust path syntax ::
  if (token.includes('://') || token.includes('::')) {
    return { type: 'free', raw: token };
  }

  const colonIdx = token.indexOf(':');
  if (colonIdx <= 0) {
    return { type: 'free', raw: token };
  }

  const rawKey = token.slice(0, colonIdx).toLowerCase().trim();
  const rawVal = token.slice(colonIdx + 1).trim();

  const normalizedKey = FILTER_ALIASES[rawKey];
  if (!normalizedKey) {
    return { type: 'free', raw: token };
  }

  if (!rawVal) {
    return { type: 'free', raw: token };
  }

  // Parse values, splitting by comma while preserving quoted segments
  const values: string[] = [];
  let current = '';
  let inQuotes = false;

  for (let j = 0; j < rawVal.length; j++) {
    const ch = rawVal[j];
    if (ch === '"' && (j === 0 || rawVal[j - 1] !== '\\')) {
      inQuotes = !inQuotes;
    } else if (ch === ',' && !inQuotes) {
      const clean = current.trim();
      if (clean) values.push(clean);
      current = '';
    } else {
      current += ch;
    }
  }
  const lastClean = current.trim();
  if (lastClean) values.push(lastClean);

  // Strip wrapping quotes from each value if present
  const unquotedValues = values
    .map((v) => {
      if (v.startsWith('"') && v.endsWith('"') && v.length >= 2) {
        return v.slice(1, -1);
      }
      return v;
    })
    .filter(Boolean);

  if (unquotedValues.length === 0) {
    return { type: 'free', raw: token };
  }

  return {
    type: 'filter',
    key: normalizedKey,
    values: unquotedValues,
    raw: token,
  };
}

/**
 * Parse rawQuery into freeText and activeFilters.
 */
export function parseQueryFilters(rawQuery: string): {
  freeText: string;
  activeFilters: ActiveFilters;
} {
  const tokens = tokenizeQuery(rawQuery);
  const freeTokens: string[] = [];
  const filtersMap: Partial<Record<FilterKey, Set<string>>> = {};

  for (const token of tokens) {
    const parsed = parseToken(token);
    if (parsed.type === 'filter' && parsed.key && parsed.values) {
      if (!filtersMap[parsed.key]) {
        filtersMap[parsed.key] = new Set<string>();
      }
      for (const val of parsed.values) {
        filtersMap[parsed.key]!.add(val);
      }
    } else {
      freeTokens.push(token);
    }
  }

  const activeFilters: ActiveFilters = {};
  for (const key of VALID_KEYS) {
    const valuesSet = filtersMap[key];
    if (valuesSet && valuesSet.size > 0) {
      activeFilters[key] = Array.from(valuesSet).join(',');
    }
  }

  return {
    freeText: freeTokens.join(' ').trim(),
    activeFilters,
  };
}

/**
 * Serialize free-text terms and active filters into a unified query string.
 */
export function buildQueryString(
  freeText: string,
  filters: ActiveFilters,
): string {
  const parts: string[] = [];
  const trimmedFree = freeText.trim();
  if (trimmedFree) {
    parts.push(trimmedFree);
  }

  for (const key of VALID_KEYS) {
    const valStr = filters[key];
    if (!valStr) continue;

    const values = valStr
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);

    if (values.length === 0) continue;

    // Deduplicate values
    const uniqueValues = Array.from(new Set(values));
    const formattedValues = uniqueValues.map((v) =>
      v.includes(' ') && !v.startsWith('"') ? `"${v}"` : v,
    );

    parts.push(`${key}:${formattedValues.join(',')}`);
  }

  return parts.join(' ').trim();
}

/**
 * Update a specific filter key in rawQuery while preserving free-text and other filters.
 */
export function setQueryFilter(
  rawQuery: string,
  key: FilterKey,
  value: string | null | undefined,
  currentFilters?: ActiveFilters,
): { newRawQuery: string; newActiveFilters: ActiveFilters } {
  const { freeText, activeFilters } = parseQueryFilters(rawQuery);
  const updatedFilters = { ...(currentFilters ?? {}), ...activeFilters };

  if (!value || !value.trim()) {
    delete updatedFilters[key];
  } else {
    // Sanitize and deduplicate
    const parts = value
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
    if (parts.length === 0) {
      delete updatedFilters[key];
    } else {
      updatedFilters[key] = Array.from(new Set(parts)).join(',');
    }
  }

  const newRawQuery = buildQueryString(freeText, updatedFilters);
  return { newRawQuery, newActiveFilters: updatedFilters };
}

/**
 * Toggle a value for a specific filter key in rawQuery.
 */
export function toggleQueryFilter(
  rawQuery: string,
  key: FilterKey,
  value: string,
  currentFilters?: ActiveFilters,
): { newRawQuery: string; newActiveFilters: ActiveFilters } {
  const cleanVal = value.trim();
  const { freeText, activeFilters } = parseQueryFilters(rawQuery);
  const baseFilters = { ...(currentFilters ?? {}), ...activeFilters };

  if (!cleanVal) {
    return { newRawQuery: rawQuery, newActiveFilters: baseFilters };
  }

  const currentStr = baseFilters[key];
  const parts = currentStr
    ? currentStr
        .split(',')
        .map((s) => s.trim())
        .filter(Boolean)
    : [];

  const exists = parts.includes(cleanVal);
  const updatedFilters = { ...baseFilters };

  if (exists) {
    const remaining = parts.filter((p) => p !== cleanVal);
    if (remaining.length === 0) {
      delete updatedFilters[key];
    } else {
      updatedFilters[key] = remaining.join(',');
    }
  } else {
    parts.push(cleanVal);
    updatedFilters[key] = parts.join(',');
  }

  const newRawQuery = buildQueryString(freeText, updatedFilters);
  return { newRawQuery, newActiveFilters: updatedFilters };
}

/**
 * Remove a specific value from a filter key in rawQuery.
 */
export function removeQueryFilter(
  rawQuery: string,
  key: FilterKey,
  value: string,
  currentFilters?: ActiveFilters,
): { newRawQuery: string; newActiveFilters: ActiveFilters } {
  const cleanVal = value.trim();
  const { freeText, activeFilters } = parseQueryFilters(rawQuery);
  const baseFilters = { ...(currentFilters ?? {}), ...activeFilters };
  const currentStr = baseFilters[key];

  if (!currentStr) {
    return { newRawQuery: rawQuery, newActiveFilters: baseFilters };
  }

  const parts = currentStr
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
  const remaining = parts.filter((p) => p !== cleanVal);
  const updatedFilters = { ...baseFilters };

  if (remaining.length === 0) {
    delete updatedFilters[key];
  } else {
    updatedFilters[key] = remaining.join(',');
  }

  const newRawQuery = buildQueryString(freeText, updatedFilters);
  return { newRawQuery, newActiveFilters: updatedFilters };
}

/**
 * Clear all filter tokens from rawQuery, preserving only free-text terms.
 */
export function clearAllQueryFilters(rawQuery: string): {
  newRawQuery: string;
  newActiveFilters: ActiveFilters;
} {
  const { freeText } = parseQueryFilters(rawQuery);
  return { newRawQuery: freeText, newActiveFilters: {} };
}
