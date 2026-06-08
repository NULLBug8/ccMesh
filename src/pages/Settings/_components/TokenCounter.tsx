import { useState } from "react";
import { toast } from "sonner";

import { TabularText } from "@/components/ui";
import { Button } from "@/components/ui/button";
import { tokensApi } from "@/services/modules/tokens";

/** Token 估算工具：输入文本，调用 count_tokens 返回近似输入 token 数。 */
export function TokenCounter() {
  const [text, setText] = useState("");
  const [result, setResult] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);

  const count = async () => {
    setLoading(true);
    try {
      const r = await tokensApi.count({
        messages: [{ role: "user", content: text }],
      });
      setResult(r.inputTokens);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <section className="flex flex-col gap-3 rounded-lg border border-edge p-5">
      <h2 className="text-sm font-medium text-ink-secondary">Token 估算</h2>
      <textarea
        className="min-h-24 rounded-sm border border-edge bg-surface-raised p-2 text-sm outline-none focus:border-edge-strong"
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder="输入文本估算 token…"
      />
      <div className="flex items-center gap-3">
        <Button size="sm" onClick={count} disabled={!text || loading}>
          估算
        </Button>
        {result !== null && (
          <span className="text-sm text-ink-secondary">
            ≈ <TabularText className="text-foreground">{result}</TabularText> tokens
          </span>
        )}
      </div>
    </section>
  );
}
