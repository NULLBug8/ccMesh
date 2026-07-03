import { webTransport } from "@/services/transport/web";
import type { AppTransport } from "@/services/transport/types";

export function isWebRuntime(): boolean {
  return true;
}

export function createTransport(): AppTransport {
  return webTransport;
}
