import { useAutoTheme } from "./hooks/useAutoTheme";
import { useThemeSync } from "./hooks/useThemeSync";
import { AppLayout } from "./layouts/AppLayout";

function App() {
  useThemeSync();
  useAutoTheme();

  return <AppLayout />;
}

export default App;
