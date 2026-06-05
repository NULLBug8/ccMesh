import { motion } from "motion/react";
import { ActivityIcon, BellIcon, SparklesIcon } from "lucide-react";
import { toast } from "sonner";

import { StatusDot, TabularText } from "@/components/ui";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { ProxyControl } from "./_components/ProxyControl";

const stats = [
  { label: "今日请求", value: "1,284", status: "success" as const },
  { label: "错误数", value: "3", status: "danger" as const },
  { label: "输出 Token", value: "98,402", status: "info" as const },
];

const stack = [
  "Tailwind CSS 4",
  "shadcn/ui",
  "Radix UI",
  "lucide-react",
  "Motion",
  "Zustand",
  "TanStack Query",
  "next-themes",
  "CodeMirror",
];

export function Dashboard() {
  return (
    <div className="mx-auto flex max-w-4xl flex-col gap-6">
      <header className="flex items-end justify-between">
        <div className="flex flex-col gap-1">
          <span className="text-[10px] font-medium tracking-[0.06em] text-ink-secondary uppercase">
            Dashboard
          </span>
          <h1 className="text-2xl font-light tracking-tight">仪表盘</h1>
          <div className="flex items-center gap-2 text-sm text-ink-secondary">
            <StatusDot status="success" pulse />
            代理在线 · 设计令牌已就绪
          </div>
        </div>
        <Button onClick={() => toast.success("Sonner 通知正常工作")}>
          <BellIcon className="size-4" /> 触发通知
        </Button>
      </header>

      <ProxyControl />

      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3 }}
        className="grid grid-cols-3 gap-4"
      >
        {stats.map((s) => (
          <Card key={s.label} className="gap-3 py-5">
            <CardContent className="flex flex-col gap-2 px-5">
              <span className="flex items-center gap-2 text-xs text-ink-secondary">
                <StatusDot status={s.status} /> {s.label}
              </span>
              <TabularText className="text-2xl text-foreground">
                {s.value}
              </TabularText>
            </CardContent>
          </Card>
        ))}
      </motion.div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <SparklesIcon className="size-5 text-primary" /> 已集成的组件
          </CardTitle>
          <CardDescription>
            技术栈与 DESIGN.md 设计令牌已集成，布局支持顶部 / 侧边两种形态
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <div className="flex flex-wrap gap-2">
            {stack.map((name) => (
              <Badge key={name} variant="success">
                {name}
              </Badge>
            ))}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="success">success</Badge>
            <Badge variant="warning">warning</Badge>
            <Badge variant="info">info</Badge>
            <Badge variant="danger">danger</Badge>
            <Badge variant="muted">idle</Badge>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button>
              <ActivityIcon className="size-4" /> Primary
            </Button>
            <Button variant="secondary">Secondary</Button>
            <Button variant="outline">Outline</Button>
            <Button variant="ghost">Ghost</Button>
            <Button variant="destructive">Destructive</Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
