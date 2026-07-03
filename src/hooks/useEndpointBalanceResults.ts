import { useQuery, useQueryClient } from "@tanstack/react-query";

import type { EndpointBalanceResult } from "@/services/modules/endpoint";

export const ENDPOINT_BALANCES_QUERY_KEY = ["endpoint-balances"] as const;
const ENDPOINT_BALANCES_STORAGE_KEY = "endpoint-balance-results";

export type EndpointBalanceResults = Record<number, EndpointBalanceResult>;

function readStoredResults(): EndpointBalanceResults {
  if (typeof window === "undefined") return {};
  try {
    const raw = window.localStorage.getItem(ENDPOINT_BALANCES_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function writeStoredResults(results: EndpointBalanceResults) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(ENDPOINT_BALANCES_STORAGE_KEY, JSON.stringify(results));
  } catch {
    // localStorage may be unavailable or full; balance display should still work in memory.
  }
}

export function useEndpointBalanceResults() {
  const queryClient = useQueryClient();
  const { data: results = {} } = useQuery({
    queryKey: ENDPOINT_BALANCES_QUERY_KEY,
    queryFn: async () => readStoredResults(),
    staleTime: Infinity,
    gcTime: Infinity,
    initialData: readStoredResults,
  });

  const setResults = (next: EndpointBalanceResults) => {
    queryClient.setQueryData<EndpointBalanceResults>(ENDPOINT_BALANCES_QUERY_KEY, (current) => {
      const merged = {
        ...(current ?? readStoredResults()),
        ...next,
      };
      writeStoredResults(merged);
      return merged;
    });
  };

  return { results, setResults };
}
