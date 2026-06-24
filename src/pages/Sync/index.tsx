import { PageLayoutEditor } from "@/components/business/page-layout/PageLayoutEditor";
import { PageSectionHost } from "@/components/business/page-layout/PageSectionHost";
import { resolveViewLayout, usePageLayoutStore } from "@/stores";
import { BackupList } from "./_components/BackupList";
import { LocalBackup } from "./_components/LocalBackup";
import { WebdavForm } from "./_components/WebdavForm";
import { syncLayoutDefinition } from "./layout";

export function Sync() {
  const savedLayout = usePageLayoutStore((state) => state.getLayout("sync"));
  const layout = resolveViewLayout(syncLayoutDefinition, savedLayout);

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6">
      <PageLayoutEditor view="sync" definition={syncLayoutDefinition} />
      <PageSectionHost
        layout={layout}
        registry={{
          header: {
            title: "标题",
            render: () => <h1 className="text-2xl font-light tracking-tight">同步</h1>,
          },
          webdav: {
            title: "WebDAV 配置",
            render: () => <WebdavForm />,
          },
          backup: {
            title: "云端备份",
            render: () => <BackupList />,
          },
          local: {
            title: "本地导入导出",
            render: () => <LocalBackup />,
          },
        }}
      />
    </div>
  );
}
