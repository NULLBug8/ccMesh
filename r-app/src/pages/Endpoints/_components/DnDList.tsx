import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { endpointApi, type Endpoint } from "@/services/modules/endpoint";
import { EndpointCard } from "./EndpointCard";

interface Props {
  endpoints: Endpoint[];
  draggable: boolean;
  onEdit: (e: Endpoint) => void;
}

/** 原生 HTML5 拖拽排序；筛选时（draggable=false）禁用。 */
export function DnDList({ endpoints, draggable, onEdit }: Props) {
  const qc = useQueryClient();
  const [order, setOrder] = useState<Endpoint[]>(endpoints);
  const [dragIdx, setDragIdx] = useState<number | null>(null);

  useEffect(() => {
    setOrder(endpoints);
  }, [endpoints]);

  const reorder = useMutation({
    mutationFn: (ids: number[]) => endpointApi.reorder(ids),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["endpoints"] }),
    onError: (e) => toast.error(e instanceof Error ? e.message : String(e)),
  });

  const onDrop = (toIdx: number) => {
    if (dragIdx === null || dragIdx === toIdx) {
      setDragIdx(null);
      return;
    }
    const next = [...order];
    const [moved] = next.splice(dragIdx, 1);
    next.splice(toIdx, 0, moved);
    setOrder(next);
    setDragIdx(null);
    reorder.mutate(next.map((e) => e.id));
  };

  return (
    <div className="flex flex-col gap-2">
      {order.map((ep, idx) => (
        <div
          key={ep.id}
          draggable={draggable}
          onDragStart={() => setDragIdx(idx)}
          onDragOver={(e) => {
            if (draggable) e.preventDefault();
          }}
          onDrop={() => {
            if (draggable) onDrop(idx);
          }}
          className={dragIdx === idx ? "opacity-50" : ""}
        >
          <EndpointCard endpoint={ep} onEdit={onEdit} draggable={draggable} />
        </div>
      ))}
    </div>
  );
}
