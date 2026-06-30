export type TransportEvent<T> = {
  payload: T;
};

export interface AppTransport {
  kind: "web";
  request<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  subscribe<T>(
    event: string,
    cb: (event: TransportEvent<T>) => void,
  ): Promise<() => void>;
}
