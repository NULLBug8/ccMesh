import type { ComponentType } from "react";
import {
  GaugeIcon,
  ServerIcon,
  FileCogIcon,
  ChartColumnIcon,
  RefreshCwIcon,
  ScrollTextIcon,
  SettingsIcon,
} from "lucide-react";

import type { ViewId } from "@/stores";

export interface NavItemDef {
  id: ViewId;
  label: string;
  labelEn: string;
  icon: ComponentType<{ className?: string }>;
}

export const NAV_ITEMS: NavItemDef[] = [
  { id: "dashboard", label: "仪表盘", labelEn: "Dashboard", icon: GaugeIcon },
  { id: "endpoints", label: "端点管理", labelEn: "Endpoints", icon: ServerIcon },
  {
    id: "configProfiles",
    label: "配置文件",
    labelEn: "Config Profiles",
    icon: FileCogIcon,
  },
  { id: "statistics", label: "统计", labelEn: "Statistics", icon: ChartColumnIcon },
  { id: "sync", label: "同步", labelEn: "Sync", icon: RefreshCwIcon },
  { id: "logs", label: "日志", labelEn: "Logs", icon: ScrollTextIcon },
];

export const SETTINGS_ITEM: NavItemDef = {
  id: "settings",
  label: "设置",
  labelEn: "Settings",
  icon: SettingsIcon,
};
