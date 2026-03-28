import { Show } from "solid-js";
import { appStore } from "./stores/app-store";
import { Sidebar } from "./components/sidebar/Sidebar";
import { TabBar } from "./components/tabs/TabBar";
import { ChatArea } from "./components/chat/ChatArea";
import { Composer } from "./components/composer/Composer";
import { SettingsOverlay } from "./components/settings/SettingsOverlay";
import { ContextMenu } from "./components/shared/ContextMenu";
import { ProviderPicker } from "./components/composer/ProviderPicker";

export function App() {
  const { store } = appStore;

  return (
    <>
      <Sidebar />
      <div class="main-panel">
        <TabBar />
        <ChatArea />
        <Composer />
      </div>

      <Show when={store.settingsOpen}>
        <SettingsOverlay />
      </Show>

      <Show when={store.providerPickerOpen}>
        <ProviderPicker />
      </Show>

      <Show when={store.contextMenu}>
        <ContextMenu />
      </Show>

      <style>{`
        .main-panel {
          flex: 1;
          display: flex;
          flex-direction: column;
          min-width: 0;
          background: var(--bg-base);
        }
      `}</style>
    </>
  );
}
