import { create } from "zustand";

import type { ProxyStatus } from "@/services/modules/proxy";

interface ProxyStoreState {
  status: ProxyStatus | null;
  setStatus: (status: ProxyStatus) => void;
}

export const useProxyStore = create<ProxyStoreState>((set) => ({
  status: null,
  setStatus: (status) => set({ status }),
}));
