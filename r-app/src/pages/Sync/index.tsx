import { BackupList } from "./_components/BackupList";
import { LocalBackup } from "./_components/LocalBackup";
import { WebdavForm } from "./_components/WebdavForm";

export function Sync() {
  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6">
      <h1 className="text-2xl font-light tracking-tight">同步</h1>
      <WebdavForm />
      <BackupList />
      <LocalBackup />
    </div>
  );
}
