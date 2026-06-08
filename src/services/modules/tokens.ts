import { request } from "../request";

export interface TokenCount {
  inputTokens: number;
}

export const tokensApi = {
  /** payload 形如 { system?, messages: [{role, content}] }。 */
  count: (payload: unknown) =>
    request<TokenCount>("count_tokens", { request: payload }),
};
