import { useQuery } from "@tanstack/react-query";

import { toolConfigApi, type AppType } from "@/services/modules/tool_config";

/** 某工具的渠道列表（读工作目录）。 */
export function useToolConfigChannels(appType: AppType) {
  return useQuery({
    queryKey: ["profile-channels", appType],
    queryFn: () => toolConfigApi.list(appType),
  });
}
