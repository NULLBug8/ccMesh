/// <reference types="vite/client" />

declare global {
  interface Window {
    __CCMESH_WEB__?: boolean;
  }
}

export {};