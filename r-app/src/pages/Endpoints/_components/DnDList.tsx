import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { DragDropProvider } from "@dnd-kit/react";
import { useSortable } from "@dnd-kit/react/sortable";
import { move } from "@dnd-kit/helpers";

import { endpointApi, type Endpoint } from "@/services/modules/endpoint";
import { EndpointCard } from "./EndpointCard";

interface Props {
  endpoints: Endpoint[];
  draggable: boolean;
  onEdit: (e: Endpoint) => void;
}

interface RowProps {
  endpoint: Endpoint;
  index: number;
  draggable: boolean;
  onEdit: (e: Endpoint) => void;
}

/** 单行：useSortable 接管位移/放置动画，把 handleRef 交给 EndpointCard 的 grip 图标。 */
function SortableRow({ endpoint, index, draggable, onEdit }: RowProps) {
  const { ref, handleRef, isDragging } = useSortable({
    id: endpoint.id,
    index,
    disabled: !draggable,
  });

  return (
    <div ref={ref} style={{ opacity: isDragging ? 0.5 : undefined }}>
      <EndpointCard
        endpoint={endpoint}
        onEdit={onEdit}
        draggable={draggable}
        dragHandleRef={handleRef}
      />
    </div>
  );
}

/** 基于 @dnd-kit/react 的拖拽排序；筛选时（draggable=false）禁用拖拽但保持渲染。 */
export function DnDList({ endpoints, draggable, onEdit }: Props) {
  const qc = useQueryClient();
  const [order, setOrder] = useState<Endpoint[]>(endpoints);

  useEffect(() => {
    setOrder(endpoints);
  }, [endpoints]);

  const reorder = useMutation({
    mutationFn: (ids: number[]) => endpointApi.reorder(ids),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["endpoints"] }),
    onError: (e) => toast.error(e instanceof Error ? e.message : String(e)),
  });

  return (
    <DragDropProvider
      onDragEnd={(event) => {
        if (event.canceled) return;
        const next = move(order, event);
        if (next.every((e, i) => e.id === order[i].id)) return;
        setOrder(next);
        reorder.mutate(next.map((e) => e.id));
      }}
    >
      <div className="flex flex-col gap-2">
        {order.map((ep, index) => (
          <SortableRow
            key={ep.id}
            endpoint={ep}
            index={index}
            draggable={draggable}
            onEdit={onEdit}
          />
        ))}
      </div>
    </DragDropProvider>
  );
}
