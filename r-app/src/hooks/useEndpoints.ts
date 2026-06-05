import { useQuery } from "@tanstack/react-query";

import { endpointApi } from "@/services/modules/endpoint";

export function useEndpoints() {
  return useQuery({ queryKey: ["endpoints"], queryFn: endpointApi.list });
}
